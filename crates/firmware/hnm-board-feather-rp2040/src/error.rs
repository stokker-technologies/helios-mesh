/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Preserve Board Setup And Satellite Failures

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::hardware::HardwareError;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T> = core::result::Result<T, Error>;

/* === Structs === */

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("hardware allocation failed: {0}")]
    Hardware(#[from] HardwareError),

    #[cfg(all(target_arch = "arm", target_os = "none"))]
    #[error("radio satellite failed: {0}")]
    Radio(#[from] crate::board::RadioUnavailable),

    #[cfg(target_arch = "arm")]
    #[error("motor satellite failed: {0}")]
    Motor(#[from] crate::wings::MotorUnavailable),
}

/* === Traits === */

/* === Implementations === */

#[cfg(target_arch = "arm")]
impl defmt::Format for Error {
    fn format(&self, formatter: defmt::Formatter<'_>) {
        defmt::write!(formatter, "{:?}", defmt::Debug2Format(self));
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
