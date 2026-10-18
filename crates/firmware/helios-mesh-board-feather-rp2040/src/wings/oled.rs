/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Electrical ownership of the oled wing.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::hardware::{HardwareError, Idle, Reservation};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Oled {
    pub button_a: u8,

    pub button_b: u8,

    pub button_c: u8,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Oled {
    #[must_use]
    pub const fn standard() -> Self {
        Self { button_a: 9, button_b: 6, button_c: 5 }
    }
}

impl Oled {
    pub(crate) fn reserve(self, reservation: &mut Reservation) -> Result<(), HardwareError> {
        for pin in [self.button_a, self.button_b, self.button_c] {
            reservation.pin(pin, Idle::PullUp)?;
        }
        reservation.i2c_address(0x3c)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
