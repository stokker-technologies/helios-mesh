/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Node chassis shared by every Helios Node Mesh firmware binary.

#![cfg_attr(not(feature = "std"), no_std)]

/* === Modules === */

pub mod boot;
pub mod diagnostics;
pub mod error;
pub mod identity;
pub mod jobs;
pub mod journal;
pub mod mesh;
pub mod node;
pub mod platform;
pub mod transport;
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
