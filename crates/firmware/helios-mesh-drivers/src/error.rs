/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Preserve Driver Failures At The Crate Boundary

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::convert::Infallible;

// Third Party Imports

// Local Imports
use crate::motor::wing::{BatchConflict, OutputUnavailable};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T, E = Infallible> = core::result::Result<T, Error<E>>;

/* === Structs === */

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error<E = Infallible> {
    // The Device Error Retains E; The HAL Does Not Require core::error::Error.
    #[error("motor device transaction failed: {0:?}")]
    Device(pwm_pca9685::Error<E>),

    #[error("motor device transaction timed out")]
    Timeout,

    #[error("motor output unavailable: {0}")]
    Output(#[from] OutputUnavailable),

    #[error("motor batch rejected: {0}")]
    Batch(#[from] BatchConflict),
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
