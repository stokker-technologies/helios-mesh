/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Statically composed wing profiles own their wiring and reservations.

/* === Modules === */

mod motor;
mod oled;
mod tft_v2;

/* === Re-Exports === */

pub use motor::Motor;

#[cfg(target_arch = "arm")]
pub use motor::{MotorResources, MotorRunner, MotorUnavailable, MotorWing};
pub use oled::Oled;
pub use tft_v2::TftV2;

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wing {
    Tft(TftV2),

    Oled(Oled),

    Motor(Motor),
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
