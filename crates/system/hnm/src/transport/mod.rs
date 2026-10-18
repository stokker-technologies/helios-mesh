/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Private USB transport: enumeration, framing, and the request driver.

/* === Modules === */

mod connection;
mod device;
pub(crate) mod framing;

/* === Re-Exports === */

pub(crate) use connection::EXTENDED_TIMEOUT;
#[cfg(any(test, feature = "testing"))]
pub use connection::MAX_IN_FLIGHT;
pub use connection::{Connection, DEFAULT_TIMEOUT};
pub use device::{UsbDevice, usb_devices};

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
