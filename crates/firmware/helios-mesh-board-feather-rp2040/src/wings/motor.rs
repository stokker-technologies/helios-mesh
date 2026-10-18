/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Electrical ownership of the motor wing.

/* === Modules === */

#[cfg(target_arch = "arm")]
mod resource {
    use crate::board::I2cBus;
    type I2cDevice =
        helios_mesh_drivers::bus::i2c1_baud::PolledI2cDevice<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex>;
    pub type MotorWing = helios_mesh_drivers::motor::wing::Wing<I2cDevice>;
    pub type MotorRunner = helios_mesh_drivers::motor::wing::WingRunner<I2cDevice>;
    #[derive(Debug, thiserror::Error)]
    #[error("motor initialization failed: {0}")]
    pub struct MotorUnavailable(#[source] pub helios_mesh_drivers::Error<<I2cDevice as embedded_hal::i2c::ErrorType>::Error>);
    pub struct MotorResources {
        i2c: &'static I2cBus,
    }
    impl MotorResources {
        pub(crate) const fn new(i2c: &'static I2cBus) -> Self {
            Self { i2c }
        }
        pub async fn connect(self) -> crate::Result<MotorWing> {
            let mut config = embassy_rp::i2c::Config::default();
            config.frequency = helios_mesh_drivers::motor::wing::MAXIMUM_I2C_FREQUENCY_HZ;
            MotorWing::connect(self.i2c.polled_device(config)).await.map_err(|error| MotorUnavailable(error).into())
        }
    }
}

/* === Re-Exports === */

#[cfg(target_arch = "arm")]
pub use resource::{MotorResources, MotorRunner, MotorUnavailable, MotorWing};

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::hardware::{HardwareError, Reservation};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Motor;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Motor {
    #[must_use]
    pub const fn standard() -> Self {
        Self
    }
}

impl Motor {
    pub(crate) fn reserve(self, reservation: &mut Reservation) -> Result<(), HardwareError> {
        let Self = self;
        reservation.i2c_address(0x60)?;
        // PCA9685 also listens to all-call by default.
        reservation.i2c_address(0x70)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
