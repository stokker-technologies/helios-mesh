/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Stable request rejection kinds shaped by the caller's corrective action.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::node::{AuxGpioError, AuxGpioId, NodeId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T> = core::result::Result<T, Error>;

/* === Structs === */

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, thiserror::Error)]
pub enum Error {
    #[error("{self:?}")]
    InvalidInput,

    #[error("{self:?}")]
    AuxGpio { gpio: AuxGpioId, reason: AuxGpioError },

    #[error("{self:?}")]
    Unsupported,

    #[error("{self:?}")]
    Unauthorized,

    #[error("{self:?}")]
    NotFound,

    #[error("{self:?}")]
    Busy,

    #[error("{self:?}")]
    Conflict,

    #[error("{self:?}")]
    BoundElsewhere { gateway: NodeId },

    #[error("{self:?}")]
    Unavailable,

    #[error("{self:?}")]
    UnsupportedVersion,

    #[error("{self:?}")]
    DuplicateRequestConflict,
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
