/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// LoRa packet delivery for Helios Node Mesh firmware.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_time::{Duration, Instant, Timer};
use lora_phy::{
    DelayNs, LoRa, RxMode,
    mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, RadioError, SpreadingFactor},
    mod_traits::RadioKind,
};

// Local Imports
use hnm_protocol::wire::mesh::{self, Envelope, MAX_PACKET_SIZE};

/* === Constants === */

pub const FREQUENCY_HZ: u32 = 869_525_000;
const PREAMBLE_SYMBOLS: u16 = 8;
const OUTPUT_POWER_DBM: i32 = 10;
const SPREADING_FACTOR: SpreadingFactor = SpreadingFactor::_7;
const IMPLICIT_HEADER: bool = false;
const HALF_DUPLEX_TURNAROUND: Duration = Duration::from_millis(2);
const SYMBOL_DURATION_US: u64 = 512;
const MIN_RECEIVE_SYMBOLS: u16 = 4;
const MAX_RECEIVE_SYMBOLS: u16 = 1_023;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendTiming {
    pub turnaround_ms: u32,

    pub preparation_ms: u32,

    pub transmit_ms: u32,
}

pub struct Link<RK, DLY>
where
    RK: RadioKind,
    DLY: DelayNs,
{
    lora: LoRa<RK, DLY>,
    modulation: ModulationParams,
    tx: PacketParams,
    rx: PacketParams,
    last_send_timing: SendTiming,
}

/* === Enums === */

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    // The Published Driver Error Implements Debug And Format, Not core::error::Error.
    #[error("radio failed: {0:?}")]
    Radio(RadioError),

    #[error("mesh packet encoding or decoding failed")]
    Packet,

    #[error("mesh receive window expired")]
    Timeout,
}

/* === Traits === */

/* === Implementations === */

impl<RK, DLY> Link<RK, DLY>
where
    RK: RadioKind,
    DLY: DelayNs,
{
    pub fn new(mut lora: LoRa<RK, DLY>) -> Result<Self, Error> {
        let modulation =
            lora.create_modulation_params(SPREADING_FACTOR, Bandwidth::_250KHz, CodingRate::_4_5, FREQUENCY_HZ).map_err(Error::Radio)?;
        let tx = lora.create_tx_packet_params(PREAMBLE_SYMBOLS, IMPLICIT_HEADER, true, false, &modulation).map_err(Error::Radio)?;
        let max_packet_size = u8::try_from(MAX_PACKET_SIZE).map_err(|_| Error::Packet)?;
        let rx = lora
            .create_rx_packet_params(PREAMBLE_SYMBOLS, IMPLICIT_HEADER, max_packet_size, true, false, &modulation)
            .map_err(Error::Radio)?;
        Ok(Self { lora, modulation, tx, rx, last_send_timing: SendTiming::default() })
    }

    pub async fn send(&mut self, packet: &Envelope) -> Result<(), Error> {
        let mut buffer = [0_u8; MAX_PACKET_SIZE];
        let encoded = mesh::encode(packet, &mut buffer).map_err(|_| Error::Packet)?;
        // A peer that just transmitted still needs time to finish IRQ processing
        // and re-enter receive mode. Apply the guard at the PHY boundary so
        // replies, acknowledgements, and follow-up requests all obey it.
        let started = Instant::now();
        Timer::after(HALF_DUPLEX_TURNAROUND).await;
        let prepared_at = Instant::now();
        self.lora.prepare_for_tx(&self.modulation, &mut self.tx, OUTPUT_POWER_DBM, encoded).await.map_err(Error::Radio)?;
        let transmitted_at = Instant::now();
        self.lora.tx().await.map_err(Error::Radio)?;
        let completed_at = Instant::now();
        self.last_send_timing = SendTiming {
            turnaround_ms: elapsed_ms(started, prepared_at),
            preparation_ms: elapsed_ms(prepared_at, transmitted_at),
            transmit_ms: elapsed_ms(transmitted_at, completed_at),
        };
        Ok(())
    }

    #[must_use]
    pub const fn last_send_timing(&self) -> SendTiming {
        self.last_send_timing
    }

    async fn receive_window(&mut self, symbols: u16) -> Result<Envelope, Error> {
        let mut buffer = [0_u8; MAX_PACKET_SIZE];
        self.lora.prepare_for_rx(RxMode::Single(symbols), &self.modulation, &self.rx).await.map_err(Error::Radio)?;
        let (length, _) = match self.lora.rx(&self.rx, &mut buffer).await {
            Ok(received) => received,
            Err(RadioError::ReceiveTimeout) => return Err(Error::Timeout),
            Err(error) => return Err(Error::Radio(error)),
        };
        mesh::decode(&buffer[..usize::from(length)]).map_err(|_| Error::Packet)
    }

    pub async fn receive_timeout(&mut self, duration: Duration) -> Result<Envelope, Error> {
        let deadline = Instant::now() + duration;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining == Duration::MIN {
                return Err(Error::Timeout);
            }

            match self.receive_window(receive_symbols(remaining)).await {
                Err(Error::Timeout) if Instant::now() < deadline => {}
                result => return result,
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn elapsed_ms(started: Instant, completed: Instant) -> u32 {
    u32::try_from(completed.duration_since(started).as_millis()).unwrap_or(u32::MAX)
}

fn receive_symbols(duration: Duration) -> u16 {
    let symbols = duration.as_micros().saturating_add(SYMBOL_DURATION_US - 1) / SYMBOL_DURATION_US;
    u16::try_from(symbols).unwrap_or(u16::MAX).clamp(MIN_RECEIVE_SYMBOLS, MAX_RECEIVE_SYMBOLS)
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receive_window_minimum() {
        assert_eq!(receive_symbols(Duration::from_micros(1)), 4);
    }

    #[test]
    fn receive_window_symbol() {
        assert_eq!(receive_symbols(Duration::from_millis(250)), 489);
        assert_eq!(receive_symbols(Duration::from_millis(600)), 1_023);
    }

    #[test]
    fn receive_window_limit() {
        assert_eq!(receive_symbols(Duration::from_secs(2)), 1_023);
    }

    #[test]
    fn variable_length_sf6() {
        assert!(SPREADING_FACTOR != SpreadingFactor::_6 || IMPLICIT_HEADER);
    }

    #[test]
    fn turnaround_retains_guard() {
        assert!(HALF_DUPLEX_TURNAROUND >= Duration::from_millis(2));
    }
}
