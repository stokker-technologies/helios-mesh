/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared I2C ownership and adapters for Helios Mesh drivers.

/* === Modules === */

/* === Re-Exports === */

pub use embassy_embedded_hal::shared_bus::I2cDeviceError;
pub use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDeviceWithConfig;

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_embedded_hal::SetConfig;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::Mutex};
use embedded_hal::i2c::I2c as BlockingI2c;
use embedded_hal_async::i2c::{self, I2c};

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct SharedI2c<M: RawMutex, BUS> {
    // `pub(crate)`: `bus::i2c1_baud` needs this to add its own, concretely
    // typed device constructor for the one physical I2C1 bus this firmware
    // has, without widening this type's own generic API.
    pub(crate) bus: Mutex<M, BUS>,
}

pub struct SharedI2cDevice<'a, M: RawMutex, BUS: SetConfig> {
    bus: &'a Mutex<M, BUS>,
    config: BUS::Config,
}

/* === Enums === */

/* === Traits === */

#[allow(async_fn_in_trait)]
pub trait I2cBatch: I2c {
    async fn write_batch(&mut self, address: u8, writes: &[&[u8]]) -> Result<(), Self::Error>;
}

/* === Implementations === */

impl<M: RawMutex, BUS> SharedI2c<M, BUS> {
    pub const fn new(bus: BUS) -> Self {
        Self { bus: Mutex::new(bus) }
    }
}

impl<M: RawMutex, BUS: SetConfig> SharedI2c<M, BUS> {
    pub fn configured_device(&self, config: BUS::Config) -> I2cDeviceWithConfig<'_, M, BUS> {
        I2cDeviceWithConfig::new(&self.bus, config)
    }

    pub fn batched_device(&self, config: BUS::Config) -> SharedI2cDevice<'_, M, BUS> {
        SharedI2cDevice { bus: &self.bus, config }
    }
}

impl<M: RawMutex, BUS: SetConfig> Clone for SharedI2cDevice<'_, M, BUS>
where
    BUS::Config: Clone,
{
    fn clone(&self) -> Self {
        Self { bus: self.bus, config: self.config.clone() }
    }
}

impl<M: RawMutex, BUS> i2c::ErrorType for SharedI2cDevice<'_, M, BUS>
where
    BUS: i2c::ErrorType + SetConfig,
{
    type Error = I2cDeviceError<BUS::Error>;
}

impl<M, BUS> I2c for SharedI2cDevice<'_, M, BUS>
where
    M: RawMutex,
    BUS: I2c + SetConfig,
{
    async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.set_config(&self.config).map_err(|_| I2cDeviceError::Config)?;
        bus.read(address, buffer).await.map_err(I2cDeviceError::I2c)
    }

    async fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.set_config(&self.config).map_err(|_| I2cDeviceError::Config)?;
        bus.write(address, bytes).await.map_err(I2cDeviceError::I2c)
    }

    async fn write_read(&mut self, address: u8, write: &[u8], read: &mut [u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.set_config(&self.config).map_err(|_| I2cDeviceError::Config)?;
        bus.write_read(address, write, read).await.map_err(I2cDeviceError::I2c)
    }

    async fn transaction(&mut self, address: u8, operations: &mut [i2c::Operation<'_>]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.set_config(&self.config).map_err(|_| I2cDeviceError::Config)?;
        bus.transaction(address, operations).await.map_err(I2cDeviceError::I2c)
    }
}

impl<M, BUS> I2cBatch for SharedI2cDevice<'_, M, BUS>
where
    M: RawMutex,
    BUS: I2c + BlockingI2c + SetConfig,
{
    async fn write_batch(&mut self, address: u8, writes: &[&[u8]]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.set_config(&self.config).map_err(|_| I2cDeviceError::Config)?;
        for write in writes {
            BlockingI2c::write(&mut *bus, address, write).map_err(I2cDeviceError::I2c)?;
        }
        Ok(())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    extern crate std;

    use core::cell::RefCell;
    use core::convert::Infallible;

    use embassy_futures::block_on;
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use std::rc::Rc;
    use std::vec::Vec;

    use super::*;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct MockConfig(u32);

    #[derive(Clone)]
    struct MockBus {
        writes: Rc<RefCell<Vec<Vec<u8>>>>,
        async_calls: Rc<RefCell<usize>>,
        applied_config: Rc<RefCell<Option<MockConfig>>>,
    }

    impl embedded_hal::i2c::ErrorType for MockBus {
        type Error = Infallible;
    }

    impl SetConfig for MockBus {
        type Config = MockConfig;
        type ConfigError = Infallible;

        fn set_config(&mut self, config: &Self::Config) -> Result<(), Self::ConfigError> {
            *self.applied_config.borrow_mut() = Some(*config);
            Ok(())
        }
    }

    impl BlockingI2c for MockBus {
        fn transaction(&mut self, _address: u8, operations: &mut [embedded_hal::i2c::Operation<'_>]) -> Result<(), Self::Error> {
            for operation in operations.iter_mut() {
                if let embedded_hal::i2c::Operation::Write(bytes) = operation {
                    self.writes.borrow_mut().push(bytes.to_vec());
                }
            }
            Ok(())
        }
    }

    impl i2c::I2c for MockBus {
        fn transaction(
            &mut self, _address: u8, _operations: &mut [i2c::Operation<'_>],
        ) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            *self.async_calls.borrow_mut() += 1;
            core::future::ready(Ok(()))
        }
    }

    #[test]
    fn write_batch_async() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let async_calls = Rc::new(RefCell::new(0));
        let applied_config = Rc::new(RefCell::new(None));
        let bus: SharedI2c<NoopRawMutex, MockBus> =
            SharedI2c::new(MockBus { writes: writes.clone(), async_calls: async_calls.clone(), applied_config: applied_config.clone() });
        let mut device = bus.batched_device(MockConfig(42));

        block_on(device.write_batch(0x60, &[&[1, 2, 3], &[4, 5]])).unwrap();

        assert_eq!(*async_calls.borrow(), 0);
        assert_eq!(*applied_config.borrow(), Some(MockConfig(42)));
        assert_eq!(writes.borrow().as_slice(), &[Vec::from([1, 2, 3]), Vec::from([4, 5])]);
    }
}
