/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Complete Feather RP2040 construction shared by every HNM node binary.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    Peri, bind_interrupts,
    clocks::RoscRng,
    flash::{self, Blocking},
    gpio::{Input, Level, Output, Pull},
    i2c::{self, Async as I2cAsync, I2c, InterruptHandler as I2cInterruptHandler},
    multicore::{Stack, spawn_core1},
    peripherals::{CORE1, DMA_CH0, DMA_CH1, DMA_CH2, FLASH, I2C1, PIO0, SPI1, USB},
    pio::{InterruptHandler as PioInterruptHandler, Pio},
    pio_programs::ws2812::{PioWs2812, PioWs2812Program},
    spi::{Async as SpiAsync, Config as SpiConfig, Spi},
    usb::{Driver, InterruptHandler as UsbInterruptHandler},
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Delay;
use lora_phy::{
    LoRa,
    sx127x::{self, Sx127x, Sx1276},
};
use static_cell::StaticCell;

// Local Imports
use crate::{
    display::{OledResources, TftResources},
    hardware::ValidatedHardware,
    pad_plan::{I2cPads, PadPlan, SpiPads},
    platform::Platform,
    radio::Interface,
    restart, status_led,
    usb::{self, UsbReader, UsbWriter},
    wings::MotorResources,
};
use hnm_core::{
    boot::{Outcome as BootOutcome, Satellite},
    identity::{self, Identity},
    mesh::link::Link,
};
use hnm_drivers::bus::{
    i2c::SharedI2c,
    spi::{HybridAsyncSpi, SharedSpi, SpiDeviceWithConfig},
};
use hnm_protocol::node::SessionId;

/* === Constants === */

const FLASH_SIZE: usize = 8 * 1024 * 1024;
const FLASH_UNIQUE_ID_SIZE: usize = 16;
const RFM_SPI_FREQUENCY_HZ: u32 = 8_000_000;
const SPI_DMA_MINIMUM_BYTES: usize = 512;
const SECONDARY_STACK_BYTES: usize = 8192;
static AUXILIARY: StaticCell<crate::gpio::State> = StaticCell::new();
static LED_SIGNALS: hnm_core::platform::led::Signals = hnm_core::platform::led::Signals::new();
static RESTART_REQUESTS: restart::Requests = restart::Requests::new();
static I2C_BUS: StaticCell<I2cBus> = StaticCell::new();
static SPI_BUS: StaticCell<SpiBus> = StaticCell::new();
static SECONDARY_EXECUTOR: StaticCell<Executor> = StaticCell::new();
static SECONDARY_STACK: StaticCell<Stack<SECONDARY_STACK_BYTES>> = StaticCell::new();

/* === Macros === */

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>, embassy_rp::dma::InterruptHandler<DMA_CH2>;
});

/* === Type Aliases === */

pub type I2cBus = SharedI2c<CriticalSectionRawMutex, I2c<'static, I2C1, I2cAsync>>;
pub type SpiBus = SharedSpi<CriticalSectionRawMutex, HybridAsyncSpi<Spi<'static, SPI1, SpiAsync>>>;
pub type Flash = flash::Flash<'static, FLASH, Blocking, FLASH_SIZE>;
pub type RadioLink = Link<RadioKind, Delay>;
pub type RadioDelay = Delay;
pub type RadioKind = Sx127x<RadioSpi, Interface<Output<'static>, Input<'static>>, Sx1276>;
type RadioSpi = SpiDeviceWithConfig<'static, CriticalSectionRawMutex, HybridAsyncSpi<Spi<'static, SPI1, SpiAsync>>, Output<'static>>;

/* === Structs === */

pub struct Board {
    pub identity: Identity,

    pub flash: Flash,

    pub usb: (UsbReader, UsbWriter),

    pub radio: Option<Radio>,

    pub platform: Platform,

    pub secondary_core: SecondaryCore,

    auxiliary: &'static crate::gpio::State,
    motor: Option<MotorResources>,
    tft: Option<TftResources>,
    oled: Option<OledResources>,
}

pub struct Radio {
    inner: RadioKind,
}

pub struct SecondaryCore {
    core: Peri<'static, CORE1>,
}

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum RadioUnavailable {
    #[error("radio initialization failed: {0:?}")]
    Driver(lora_phy::mod_params::RadioError),

    #[error("radio link configuration failed: {0}")]
    Link(#[from] hnm_core::mesh::link::Error),
}

/* === Traits === */

/* === Implementations === */

impl Board {
    #[expect(clippy::unused_async, reason = "construction is part of the async composition root contract")]
    #[allow(clippy::too_many_lines, reason = "explicit physical construction order is safety-critical")]
    #[expect(clippy::unused_async_trait_impl, reason = "Keep hardware initialization lazy within the asynchronous board boot sequence.")]
    pub async fn start(spawner: Spawner, product: &'static str, hardware: ValidatedHardware) -> Self {
        let peripherals = embassy_rp::init(embassy_rp::config::Config::default());

        let (spi_pads, i2c_pads, mut header) = PadPlan::quiesce(
            &hardware,
            peripherals.PIN_0,
            peripherals.PIN_1,
            peripherals.PIN_2,
            peripherals.PIN_3,
            peripherals.PIN_5,
            peripherals.PIN_6,
            peripherals.PIN_8,
            peripherals.PIN_9,
            peripherals.PIN_10,
            peripherals.PIN_11,
            peripherals.PIN_12,
            peripherals.PIN_13,
            peripherals.PIN_14,
            peripherals.PIN_15,
            peripherals.PIN_24,
            peripherals.PIN_25,
            peripherals.PIN_26,
            peripherals.PIN_27,
            peripherals.PIN_28,
            peripherals.PIN_29,
        );

        let mut flash = flash::Flash::<_, Blocking, FLASH_SIZE>::new_blocking(peripherals.FLASH);
        let mut unique_id = [0; FLASH_UNIQUE_ID_SIZE];
        defmt::unwrap!(flash.blocking_unique_id(&mut unique_id));
        let identity = Identity { node: identity::from_hardware_unique_id(&unique_id), session: session_id() };

        let (led, led_listener) = LED_SIGNALS.split();
        let Pio { mut common, sm0, .. } = Pio::new(peripherals.PIO0, Irqs);
        let program = PioWs2812Program::new(&mut common);
        let pixel = PioWs2812::new(&mut common, sm0, peripherals.DMA_CH0, Irqs, peripherals.PIN_4, &program);
        spawner.spawn(defmt::unwrap!(status_led::run(pixel, led_listener)));
        spawner.spawn(defmt::unwrap!(restart::run(peripherals.WATCHDOG, &RESTART_REQUESTS)));

        let driver = Driver::new(peripherals.USB, Irqs);
        let (device, usb) = usb::build(driver, identity.node, product);
        spawner.spawn(defmt::unwrap!(usb::device_task(device)));

        let mut radio_config = SpiConfig::default();
        radio_config.frequency = RFM_SPI_FREQUENCY_HZ;
        let radio_pins = hardware.buses_enabled().then(|| {
            (
                Output::new(peripherals.PIN_16, Level::High),
                Output::new(peripherals.PIN_17, Level::High),
                Input::new(peripherals.PIN_21, Pull::None),
                Input::new(peripherals.PIN_22, Pull::None),
            )
        });
        let spi = spi_pads.map(|pads| start_spi(peripherals.SPI1, pads, peripherals.DMA_CH1, peripherals.DMA_CH2, radio_config.clone()));
        let radio = spi.zip(radio_pins).map(|(spi, (cs, reset, dio0, dio1))| radio(spi, radio_config, cs, reset, dio0, dio1));
        let i2c = i2c_pads.map(|pads| start_i2c(peripherals.I2C1, pads));
        let tft =
            hardware.tft().map(|wiring| TftResources::new(&mut header, wiring, spi.expect("validated SPI"), i2c.expect("validated I2C")));
        let oled = hardware.oled().map(|wiring| OledResources::new(&mut header, wiring, i2c.expect("validated I2C")));
        let motor = hardware.motor().then(|| MotorResources::new(i2c.expect("declared motor I2C")));
        let auxiliary = AUXILIARY.init(crate::gpio::State::new(crate::auxiliary::AuxiliaryGpio::new(hardware, |pin| header.take(pin))));

        let platform = Platform::new(auxiliary, &RESTART_REQUESTS, led);

        Self {
            identity,
            flash,
            usb,
            radio,
            platform,
            secondary_core: SecondaryCore { core: peripherals.CORE1 },
            auxiliary,
            motor,
            tft,
            oled,
        }
    }

    pub fn take_auxiliary(&mut self, id: hnm_protocol::node::AuxGpioId) -> Result<crate::AuxiliaryPin, hnm_protocol::error::Error> {
        self.auxiliary.take(id)
    }

    pub fn take_motor(&mut self) -> Option<MotorResources> {
        self.motor.take()
    }
    pub fn take_tft(&mut self) -> Option<TftResources> {
        self.tft.take()
    }
    pub fn take_oled(&mut self) -> Option<OledResources> {
        self.oled.take()
    }
}

impl Radio {
    async fn link(self) -> crate::Result<RadioLink> {
        let lora = LoRa::new(self.inner, false, Delay).await.map_err(RadioUnavailable::Driver)?;
        Link::new(lora).map_err(|error| RadioUnavailable::Link(error).into())
    }
}

impl Satellite for Radio {
    const NAME: hnm_protocol::satellite::SatelliteName = hnm_protocol::satellite::SatelliteName::Radio;

    type Ready = RadioLink;
    type Fault = crate::Error;

    async fn bring_up(self) -> BootOutcome<Self::Ready, Self::Fault> {
        match self.link().await {
            Ok(link) => BootOutcome::Ready(link),
            Err(error) => BootOutcome::Faulted(error),
        }
    }
}

impl SecondaryCore {
    pub fn start<F>(self, spawn: F)
    where
        F: FnOnce(Spawner) + Send + 'static,
    {
        spawn_core1(self.core, SECONDARY_STACK.init(Stack::new()), move || SECONDARY_EXECUTOR.init(Executor::new()).run(spawn));
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn start_spi(
    peripheral: Peri<'static, SPI1>, pads: SpiPads, transmit_dma: Peri<'static, DMA_CH1>, receive_dma: Peri<'static, DMA_CH2>,
    config: SpiConfig,
) -> &'static SpiBus {
    let (sck, mosi, miso) = pads.into_parts();
    let spi = Spi::new(peripheral, sck, mosi, miso, transmit_dma, receive_dma, Irqs, config);
    &*SPI_BUS.init(SharedSpi::new(HybridAsyncSpi::new(spi, SPI_DMA_MINIMUM_BYTES)))
}

fn start_i2c(peripheral: Peri<'static, I2C1>, pads: I2cPads) -> &'static I2cBus {
    let (scl, sda) = pads.into_parts();
    let i2c = I2c::new_async(peripheral, scl, sda, Irqs, i2c::Config::default());
    // Embassy 0.10 omits IC_CON.TX_EMPTY_CTRL initialization. Enable the
    // shift-register-empty handshake before publishing the idle bus.
    let registers = rp_pac::I2C1;
    registers.ic_enable().write(|w| w.set_enable(false));
    registers.ic_con().modify(|w| w.set_tx_empty_ctrl(true));
    registers.ic_enable().write(|w| w.set_enable(true));
    &*I2C_BUS.init(SharedI2c::new(i2c))
}

fn session_id() -> SessionId {
    let mut rng = RoscRng;
    loop {
        let mut bytes = [0; 2];
        rng.fill_bytes(&mut bytes);
        let session = SessionId(u16::from_le_bytes(bytes));
        if session != SessionId::UNKNOWN {
            return session;
        }
    }
}

fn radio(
    spi: &'static SpiBus, config: SpiConfig, chip_select: Output<'static>, reset: Output<'static>, dio0: Input<'static>,
    dio1: Input<'static>,
) -> Radio {
    let interface = Interface::new(reset, dio0, dio1);
    Radio {
        inner: Sx127x::new(
            spi.configured_device(chip_select, config),
            interface,
            sx127x::Config { chip: Sx1276, tcxo_used: false, tx_boost: true, rx_boost: false },
        ),
    }
}

/* === Tests === */
