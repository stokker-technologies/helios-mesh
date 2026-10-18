/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// CLI errors and their stable process exit codes.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T> = core::result::Result<T, Error>;

/* === Structs === */

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid environment configuration: {0}")]
    Environment(#[source] dotenvy::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    HeliosMesh(#[from] helios_mesh::Error),

    #[error("invalid six-digit node id: {0}")]
    InvalidNodeId(String),

    #[error("{0}")]
    Bench(String),

    #[error("{0}")]
    Flash(String),
}

/* === Traits === */

/* === Implementations === */

impl Error {
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Environment(_) | Self::InvalidNodeId(_) | Self::HeliosMesh(helios_mesh::Error::InvalidArgument(_)) => 2,
            Self::Io(_) | Self::Json(_) | Self::HeliosMesh(_) | Self::Bench(_) | Self::Flash(_) => 3,
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_failure() {
        assert_eq!(Error::InvalidNodeId("x".into()).exit_code(), 2);
        assert_eq!(Error::HeliosMesh(helios_mesh::Error::InvalidArgument("not rotational")).exit_code(), 2);
        for error in [
            helios_mesh::Error::NotFound,
            helios_mesh::Error::Busy,
            helios_mesh::Error::Timeout,
            helios_mesh::Error::Unavailable,
            helios_mesh::Error::Unsupported,
        ] {
            assert_eq!(Error::HeliosMesh(error).exit_code(), 3);
        }
        assert_eq!(Error::Flash("volume".into()).exit_code(), 3);
    }
}
