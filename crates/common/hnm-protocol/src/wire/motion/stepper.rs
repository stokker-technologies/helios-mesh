/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Boot-local configuration for stepper-driven motion channels.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Mechanics {
    pub full_steps: u16,

    pub microsteps: Microsteps,

    pub reversed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Limits {
    pub speed: u16,

    pub acceleration: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub moving: u8,

    pub holding: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StepperConfiguration {
    pub mechanics: Mechanics,

    pub limits: Limits,

    pub power: Power,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Microsteps {
    Four,

    Eight,
}

/* === Traits === */

/* === Implementations === */

impl Microsteps {
    #[must_use]
    pub const fn count(self) -> u32 {
        match self {
            Self::Four => 4,
            Self::Eight => 8,
        }
    }
}

impl StepperConfiguration {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.mechanics.full_steps > 0
            && self.limits.speed > 0
            && self.limits.acceleration > 0
            && self.power.moving > 0
            && self.power.moving <= 100
            && self.power.holding <= 100
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_definitions() {
        let valid = StepperConfiguration {
            mechanics: Mechanics { full_steps: 200, microsteps: Microsteps::Eight, reversed: false },
            limits: Limits { speed: 60, acceleration: 180 },
            power: Power { moving: 25, holding: 10 },
        };
        assert!(valid.is_valid());
        assert!(!StepperConfiguration { limits: Limits { speed: 0, ..valid.limits }, ..valid }.is_valid());
        assert!(!StepperConfiguration { power: Power { moving: 101, ..valid.power }, ..valid }.is_valid());
    }
}
