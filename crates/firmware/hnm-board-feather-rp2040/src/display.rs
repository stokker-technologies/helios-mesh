/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// One-shot display capabilities preserve reservations even when probes fail.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::{convert::Infallible, mem::ManuallyDrop};

// Third Party Imports
use embassy_rp::{gpio::Flex, i2c, spi};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin};

// Local Imports
use crate::{
    board::{I2cBus, SpiBus},
    pad_plan::Header,
    wings::{Oled, TftV2},
};
use hnm_drivers::{
    bus::{
        i2c1_baud::PolledI2cDevice,
        spi::{HybridAsyncSpi, SpiDeviceWithConfig},
    },
    display,
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type I2cDevice = PolledI2cDevice<'static, CriticalSectionRawMutex>;
type SpiDevice = SpiDeviceWithConfig<
    'static,
    CriticalSectionRawMutex,
    HybridAsyncSpi<spi::Spi<'static, embassy_rp::peripherals::SPI1, spi::Async>>,
    DisplayOutput,
>;
pub type TftPanel = display::tft::Tft<SpiDevice, DisplayOutput, I2cDevice>;
pub type OledPanel = display::oled::Oled<I2cDevice, DisplayInput, DisplayInput>;

/* === Structs === */

pub struct DisplayInput(ManuallyDrop<Flex<'static>>);

pub struct DisplayOutput(ManuallyDrop<Flex<'static>>);

pub struct TftResources {
    cs: ManuallyDrop<Flex<'static>>,
    dc: ManuallyDrop<Flex<'static>>,
    i2c: &'static I2cBus,
    spi: &'static SpiBus,
}

pub struct OledResources {
    a: DisplayInput,
    c: DisplayInput,
    i2c: &'static I2cBus,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl ErrorType for DisplayInput {
    type Error = Infallible;
}

impl InputPin for DisplayInput {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.is_high())
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.is_low())
    }
}

impl ErrorType for DisplayOutput {
    type Error = Infallible;
}

impl OutputPin for DisplayOutput {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_high();
        Ok(())
    }
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_low();
        Ok(())
    }
}

impl TftResources {
    pub(crate) fn new(header: &mut Header, wiring: TftV2, spi: &'static SpiBus, i2c: &'static I2cBus) -> Self {
        // SD CS and touch IRQ remain permanently idled in the header owner.
        Self { cs: header.take(wiring.cs), dc: header.take(wiring.dc), i2c, spi }
    }
    pub async fn detect(&mut self) -> bool {
        embassy_time::Timer::after_millis(50).await;
        display::tft::detect(&mut self.i2c.polled_device(i2c_config())).await
    }
    #[must_use]
    pub fn into_panel(mut self) -> TftPanel {
        self.cs.set_high();
        self.cs.set_as_output();
        self.dc.set_low();
        self.dc.set_as_output();
        let mut config = spi::Config::default();
        config.frequency = display::tft::MAXIMUM_SPI_FREQUENCY_HZ;
        display::tft::Tft::new(
            self.spi.configured_device(DisplayOutput(self.cs), config),
            DisplayOutput(self.dc),
            self.i2c.polled_device(i2c_config()),
        )
    }
}

impl OledResources {
    pub(crate) fn new(header: &mut Header, wiring: Oled, i2c: &'static I2cBus) -> Self {
        Self { a: DisplayInput(header.take(wiring.button_a)), c: DisplayInput(header.take(wiring.button_c)), i2c }
    }
    pub async fn detect(&mut self) -> bool {
        embassy_time::Timer::after_millis(50).await;
        display::oled::detect(&mut self.i2c.polled_device(i2c_config())).await
    }
    #[must_use]
    pub fn into_panel(self) -> OledPanel {
        display::oled::Oled::new(self.i2c.polled_device(i2c_config()), self.a, self.c)
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn i2c_config() -> i2c::Config {
    let mut config = i2c::Config::default();
    config.frequency = 400_000;
    config
}

/* === Tests === */
