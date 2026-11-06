/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Model Packet Arrivals At The Radio Chip Boundary Without Physical RF.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{cell::RefCell, collections::VecDeque, future::Future, rc::Rc};

// Third Party Imports
use embassy_time::{Duration, Instant, Timer};
use embedded_hal_async::{
    delay::DelayNs,
    spi::{Operation, SpiDevice},
};
use lora_phy::{
    LoRa, RxMode,
    mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, PacketStatus, RadioError, RadioMode, SpreadingFactor},
    mod_traits::{InterfaceVariant, IrqState, RadioKind},
    sx127x::{Config, Sx127x, Sx1276},
};

// Local Imports
use helios_mesh_core::mesh::link::Link;
use helios_mesh_protocol::wire::mesh::{self, Envelope};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type ReplyScript = dyn FnMut(&Envelope, &mut VecDeque<Arrival>);

/* === Structs === */

pub struct Arrival {
    pub at: Instant,
    pub bytes: Vec<u8>,
}

pub struct Air {
    pub sent: Vec<(Instant, Envelope)>,
    pub arrivals: VecDeque<Arrival>,
    pub script: Box<ReplyScript>,
}

pub struct Radio {
    pub air: Rc<RefCell<Air>>,
    parameters: Sx127x<NoSpi, NoInterface, Sx1276>,
    payload: Vec<u8>,
    received: Vec<u8>,
    timeout: Instant,
    transmitting: bool,
    timed_out: bool,
}

pub struct Delay;
struct NoSpi;
struct NoInterface;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Arrival {
    pub fn packet(delay_ms: u64, packet: &Envelope) -> Self {
        let mut buffer = [0; mesh::MAX_PACKET_SIZE];
        let bytes = mesh::encode(packet, &mut buffer).unwrap().to_vec();
        assert_eq!(mesh::decode(&bytes).unwrap(), *packet);
        Self { at: Instant::now() + Duration::from_millis(delay_ms), bytes }
    }

    pub fn corrupt(delay_ms: u64) -> Self {
        Self { at: Instant::now() + Duration::from_millis(delay_ms), bytes: vec![0xff] }
    }
}

impl DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        Timer::after(Duration::from_nanos(u64::from(ns))).await;
    }
}

impl embedded_hal::spi::ErrorType for NoSpi {
    type Error = core::convert::Infallible;
}

impl SpiDevice for NoSpi {
    fn transaction(&mut self, _: &mut [Operation<'_, u8>]) -> impl Future<Output = Result<(), Self::Error>> {
        core::future::poll_fn(|_| unreachable!("only the driver's parameter constructors are used"))
    }
}

impl InterfaceVariant for NoInterface {
    fn reset(&mut self, _: &mut impl DelayNs) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn wait_on_busy(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn await_irq(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn enable_rf_switch_rx(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn enable_rf_switch_tx(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn disable_rf_switch(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
}

impl RadioKind for Radio {
    fn init_lora(&mut self, _: bool) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn create_modulation_params(
        &self, spreading: SpreadingFactor, bandwidth: Bandwidth, coding: CodingRate, frequency: u32,
    ) -> Result<ModulationParams, RadioError> {
        self.parameters.create_modulation_params(spreading, bandwidth, coding, frequency)
    }
    fn create_packet_params(
        &self, preamble: u16, implicit: bool, length: u8, crc: bool, inverted: bool, modulation: &ModulationParams,
    ) -> Result<PacketParams, RadioError> {
        self.parameters.create_packet_params(preamble, implicit, length, crc, inverted, modulation)
    }
    fn reset(&mut self, _: &mut impl DelayNs) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn ensure_ready(&mut self, _: RadioMode) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_standby(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_sleep(&mut self, _: bool, _: &mut impl DelayNs) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_tx_rx_buffer_base_address(&mut self, _: usize, _: usize) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_tx_power_and_ramp_time(
        &mut self, _: i32, _: Option<&ModulationParams>, _: bool,
    ) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_modulation_params(&mut self, _: &ModulationParams) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_packet_params(&mut self, _: &PacketParams) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn calibrate_image(&mut self, _: u32) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_channel(&mut self, _: u32) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_payload(&mut self, payload: &[u8]) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready({
            self.payload = payload.to_vec();
            Ok(())
        })
    }
    fn do_tx(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready({
            let packet = mesh::decode(&self.payload).unwrap();
            let mut air = self.air.borrow_mut();
            air.sent.push((Instant::now(), packet.clone()));
            let Air { script, arrivals, .. } = &mut *air;
            script(&packet, arrivals);
            arrivals.make_contiguous().sort_by_key(|arrival| arrival.at);
            self.transmitting = true;
            self.timed_out = false;
            Ok(())
        })
    }
    fn do_rx(&mut self, mode: RxMode) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready({
            let RxMode::Single(symbols) = mode else { unreachable!("the production Link uses single receive windows") };
            self.timeout = Instant::now() + Duration::from_micros(u64::from(symbols) * 512);
            self.transmitting = false;
            self.timed_out = false;
            // Frames Sent While The Radio Was Not Listening Are Lost.
            self.air.borrow_mut().arrivals.retain(|arrival| arrival.at >= Instant::now());
            Ok(())
        })
    }
    fn get_rx_payload(&mut self, _: &PacketParams, buffer: &mut [u8]) -> impl Future<Output = Result<u8, RadioError>> {
        core::future::ready({
            buffer[..self.received.len()].copy_from_slice(&self.received);
            Ok(u8::try_from(self.received.len()).unwrap())
        })
    }
    fn get_rx_packet_status(&mut self) -> impl Future<Output = Result<PacketStatus, RadioError>> {
        core::future::ready(Ok(PacketStatus { rssi: -70, snr: 10 }))
    }
    fn do_cad(&mut self, _: &ModulationParams) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    fn set_irq_params(&mut self, _: Option<RadioMode>) -> impl Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
    fn set_tx_continuous_wave_mode(&mut self) -> impl Future<Output = Result<(), RadioError>> {
        core::future::poll_fn(|_| unreachable!())
    }
    async fn await_irq(&mut self) -> Result<(), RadioError> {
        if self.transmitting {
            return Ok(());
        }
        let arrival = self.air.borrow().arrivals.front().map(|arrival| arrival.at);
        let deadline = arrival.map_or(self.timeout, |at| at.min(self.timeout));
        Timer::at(deadline).await;
        if let Some(arrival) = arrival.filter(|at| *at <= self.timeout) {
            assert!(Instant::now() >= arrival);
            self.received = self.air.borrow_mut().arrivals.pop_front().unwrap().bytes;
        } else {
            self.timed_out = true;
        }
        Ok(())
    }
    fn process_irq_event(
        &mut self, _: RadioMode, _: Option<&mut bool>, _: bool,
    ) -> impl Future<Output = Result<Option<IrqState>, RadioError>> {
        core::future::ready(if self.timed_out { Err(RadioError::ReceiveTimeout) } else { Ok(Some(IrqState::Done)) })
    }
}

/* === Public Functions === */

pub async fn link(air: Rc<RefCell<Air>>) -> Link<Radio, Delay> {
    let parameters = Sx127x::new(NoSpi, NoInterface, Config { chip: Sx1276, tcxo_used: false, tx_boost: true, rx_boost: false });
    let radio = Radio {
        air,
        parameters,
        payload: Vec::new(),
        received: Vec::new(),
        timeout: Instant::now(),
        transmitting: false,
        timed_out: false,
    };
    Link::new(LoRa::new(radio, false, Delay).await.unwrap()).unwrap()
}

/* === Private Functions === */

/* === Tests === */
