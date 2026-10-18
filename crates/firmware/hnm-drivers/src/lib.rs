/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Constructed driver objects behind `embedded-hal` traits for HNM firmware.

#![no_std]

/* === Modules === */

pub mod bus;
pub mod display;
pub mod error;
pub mod motor;

/* === Re-Exports === */

pub use error::{Error, Result};

pub use motor::wing as motor_wing;

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
