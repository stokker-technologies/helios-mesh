/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Node definitions and shared composition helpers for every HNM node binary.

#![no_std]

/* === Modules === */

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod composition;
pub mod error;
pub mod nodes;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod satellites;
pub mod ui;

/* === Re-Exports === */

pub use error::{Error, Result};

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
