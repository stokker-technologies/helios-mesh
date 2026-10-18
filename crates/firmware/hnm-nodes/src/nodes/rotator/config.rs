/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Validated in-memory rotator motor configuration.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm::motion::{Configuration, StepperConfiguration};
use hnm::motor;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    pub steppers: [StepperConfiguration; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidConfiguration;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Config {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.steppers.iter().all(|stepper| stepper.is_valid())
    }

    #[must_use]
    pub fn motor(&self, index: usize) -> motor::Config {
        let stepper = self.steppers[index];
        motor::Config { mechanics: stepper.mechanics, power: stepper.power }
    }
}

impl TryFrom<&Configuration> for Config {
    type Error = InvalidConfiguration;

    fn try_from(configuration: &Configuration) -> Result<Self, Self::Error> {
        let steppers = <[StepperConfiguration; 2]>::try_from(configuration.steppers.as_slice()).map_err(|_| InvalidConfiguration)?;
        let config = Self { steppers };
        if config.is_valid() { Ok(config) } else { Err(InvalidConfiguration) }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm::motion::{Limits, Mechanics, Microsteps, Power};

    use super::*;

    fn stepper() -> StepperConfiguration {
        StepperConfiguration {
            mechanics: Mechanics { full_steps: 200, microsteps: Microsteps::Eight, reversed: false },
            limits: Limits { speed: 60, acceleration: 180 },
            power: Power { moving: 25, holding: 10 },
        }
    }

    fn configuration(steppers: &[StepperConfiguration]) -> Configuration {
        Configuration { steppers: heapless::Vec::from_slice(steppers).unwrap() }
    }

    #[test]
    fn accepts_exactly_steppers() {
        let config = Config::try_from(&configuration(&[stepper(); 2])).unwrap();
        assert_eq!(config.steppers, [stepper(); 2]);
        assert!(config.is_valid());
    }

    #[test]
    fn rejects_wrong_definitions() {
        assert_eq!(Config::try_from(&configuration(&[stepper()])), Err(InvalidConfiguration));
        let mut broken = stepper();
        broken.mechanics.full_steps = 0;
        assert_eq!(Config::try_from(&configuration(&[stepper(), broken])), Err(InvalidConfiguration));
    }

    #[test]
    fn motor_definitions_power() {
        let config = Config { steppers: [stepper(); 2] };
        let motor = config.motor(1);
        assert_eq!(motor.mechanics, stepper().mechanics);
        assert_eq!(motor.power, stepper().power);
    }
}
