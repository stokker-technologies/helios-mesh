/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Chassis-owned display read model, activity, and navigation.

/* === Modules === */

mod activity;
mod model;

/* === Re-Exports === */

pub use activity::{ActivityDirection, ActivityState, RequestActivity};
pub use model::{LocalPage, MAX_NEARBY_DEVICES, NearbyPage, RoundTripStats, RoundTripWindow, Snapshot};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch};

// Local Imports

/* === Constants === */

pub const MAX_UI_RECEIVERS: usize = 2;

/* === Macros === */

/* === Type Aliases === */

pub(crate) type Channel = watch::Watch<CriticalSectionRawMutex, Snapshot, MAX_UI_RECEIVERS>;
pub type Receiver = watch::Receiver<'static, CriticalSectionRawMutex, Snapshot, MAX_UI_RECEIVERS>;

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
