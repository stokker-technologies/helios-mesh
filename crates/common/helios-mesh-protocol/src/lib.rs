/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Versioned wire contract shared by Helios Mesh firmware and hosts.

#![cfg_attr(not(feature = "std"), no_std)]

/* === Modules === */

#[cfg(feature = "std")]
pub mod call;
pub mod codec;
pub mod diagnostics;
pub mod error;
pub mod nodes;
pub mod satellite;
pub mod usb;
pub mod wire;

/* === Re-Exports === */

pub use error::{Error, Result};
pub use wire::{gateway, motion, node};

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use crate::{
        node::{AuxGpioId, FIRMWARE_VERSION},
        wire::PROTOCOL_VERSION,
    };

    #[test]
    fn versions_match() {
        assert_eq!(PROTOCOL_VERSION, 14);
        let major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
        let minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
        let patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u8>().unwrap();
        assert_eq!((FIRMWARE_VERSION.major, FIRMWARE_VERSION.minor, FIRMWARE_VERSION.patch), (major, minor, patch));
    }

    #[test]
    fn auxiliary_gpio_bounded() {
        assert!(!AuxGpioId(0).is_valid());
        assert!(AuxGpioId(1).is_valid());
        assert!(AuxGpioId(4).is_valid());
        assert!(!AuxGpioId(5).is_valid());
    }
}
