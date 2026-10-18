/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared SPI ownership and adapters for HNM drivers.

/* === Modules === */

/* === Re-Exports === */

pub use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_embedded_hal::SetConfig;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::Mutex};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorType, SpiBus as BlockingSpiBus};
use embedded_hal_async::spi::SpiBus as AsyncSpiBus;

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct SharedSpi<M: RawMutex, BUS> {
    bus: Mutex<M, BUS>,
}

pub struct HybridAsyncSpi<T> {
    inner: T,
    dma_minimum_bytes: usize,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<M: RawMutex, BUS> SharedSpi<M, BUS> {
    pub const fn new(bus: BUS) -> Self {
        Self { bus: Mutex::new(bus) }
    }
}

impl<M: RawMutex, BUS: SetConfig> SharedSpi<M, BUS> {
    pub fn configured_device<CS: OutputPin>(&self, chip_select: CS, config: BUS::Config) -> SpiDeviceWithConfig<'_, M, BUS, CS> {
        SpiDeviceWithConfig::new(&self.bus, chip_select, config)
    }
}

impl<T> HybridAsyncSpi<T> {
    pub const fn new(inner: T, dma_minimum_bytes: usize) -> Self {
        Self { inner, dma_minimum_bytes }
    }

    const fn use_dma(&self, bytes: usize) -> bool {
        bytes >= self.dma_minimum_bytes
    }
}

impl<T: ErrorType> ErrorType for HybridAsyncSpi<T> {
    type Error = T::Error;
}

impl<T> AsyncSpiBus<u8> for HybridAsyncSpi<T>
where
    T: BlockingSpiBus<u8> + AsyncSpiBus<u8>,
{
    async fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        if self.use_dma(words.len()) {
            AsyncSpiBus::read(&mut self.inner, words).await
        } else {
            BlockingSpiBus::read(&mut self.inner, words)
        }
    }

    async fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        if self.use_dma(words.len()) {
            AsyncSpiBus::write(&mut self.inner, words).await
        } else {
            BlockingSpiBus::write(&mut self.inner, words)
        }
    }

    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        if self.use_dma(read.len().max(write.len())) {
            AsyncSpiBus::transfer(&mut self.inner, read, write).await
        } else {
            BlockingSpiBus::transfer(&mut self.inner, read, write)
        }
    }

    async fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        if self.use_dma(words.len()) {
            AsyncSpiBus::transfer_in_place(&mut self.inner, words).await
        } else {
            BlockingSpiBus::transfer_in_place(&mut self.inner, words)
        }
    }

    fn flush(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        core::future::poll_fn(|_| core::task::Poll::Ready(BlockingSpiBus::flush(&mut self.inner)))
    }
}

impl<T: SetConfig> SetConfig for HybridAsyncSpi<T> {
    type Config = T::Config;
    type ConfigError = T::ConfigError;

    fn set_config(&mut self, config: &Self::Config) -> Result<(), Self::ConfigError> {
        self.inner.set_config(config)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
