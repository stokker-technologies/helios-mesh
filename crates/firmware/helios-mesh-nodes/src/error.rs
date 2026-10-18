/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Return Composition Failures To The Firmware Entry Point

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
    #[error("firmware composition failed: {0}")]
    Boot(#[from] helios_mesh_core::Error),
}

/* === Traits === */

/* === Implementations === */

impl defmt::Format for Error {
    fn format(&self, formatter: defmt::Formatter<'_>) {
        defmt::write!(formatter, "{:?}", defmt::Debug2Format(self));
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
