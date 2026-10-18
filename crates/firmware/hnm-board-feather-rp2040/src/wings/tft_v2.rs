/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Electrical ownership of the tft v2 wing.

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
pub struct TftV2 {
    pub cs: u8,

    pub dc: u8,

    pub touch_irq: u8,

    pub sd_cs: u8,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl TftV2 {
    #[must_use]
    pub const fn standard() -> Self {
        Self { cs: 9, dc: 10, touch_irq: 6, sd_cs: 5 }
    }
}

impl TftV2 {
    pub(crate) fn reserve(self, reservation: &mut Reservation) -> Result<(), HardwareError> {
        reservation.pin(self.cs, Idle::PullUp)?;
        reservation.pin(self.dc, Idle::HighImpedance)?;
        reservation.pin(self.touch_irq, Idle::PullUp)?;
        reservation.pin(self.sd_cs, Idle::PullUp)?;
        reservation.i2c_address(0x48)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
