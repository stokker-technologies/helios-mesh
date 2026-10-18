/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Board support for the Adafruit Feather RP2040 RFM95 used by HNM firmware.

#![no_std]

/* === Modules === */

pub mod error;
pub mod hardware;
pub mod radio;
pub mod wings;

#[cfg(any(target_arch = "arm", test))]
mod auxiliary;

#[cfg(target_arch = "arm")]
mod gpio;

#[cfg(target_arch = "arm")]
mod board;

#[cfg(target_arch = "arm")]
mod display;

#[cfg(target_arch = "arm")]
mod pad_plan;

#[cfg(target_arch = "arm")]
mod platform;

#[cfg(target_arch = "arm")]
mod restart;

#[cfg(target_arch = "arm")]
mod status_led;

#[cfg(target_arch = "arm")]
mod usb;

/* === Re-Exports === */

pub use error::{Error, Result};

pub use hardware::{Auxiliary, Drive, Hardware, HardwareError, ValidatedHardware};
pub use wings::Wing;

#[cfg(target_arch = "arm")]
pub use board::{Board, Flash, I2cBus, Radio, RadioDelay, RadioKind, RadioLink, RadioUnavailable, SecondaryCore, SpiBus};

#[cfg(target_arch = "arm")]
pub use display::{OledPanel, OledResources, TftPanel, TftResources};

#[cfg(target_arch = "arm")]
pub use platform::Platform;

#[cfg(target_arch = "arm")]
pub use gpio::AuxiliaryPin;

#[cfg(target_arch = "arm")]
pub use usb::{UsbReader, UsbWriter};

#[cfg(target_arch = "arm")]
pub use wings::{MotorResources, MotorRunner, MotorUnavailable, MotorWing};

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
