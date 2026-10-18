/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared identity, status, GPIO payloads, and operation lifecycle requests.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::fmt;

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::{
    diagnostics::{JournalPage, NodeDiagnostics},
    satellite::{MAX_SATELLITES, SatelliteReport},
};

/* === Constants === */

pub const MAX_CHANNELS: usize = 4;
pub const MAX_NODE_LABEL: usize = 24;
pub const AUX_GPIO_COUNT: usize = 4;
const AUX_GPIO_MAX_ID: u8 = 4;
pub const MAX_AUX_GPIO_PULSE_MS: u32 = 60_000;

/* === Macros === */

include!(concat!(env!("OUT_DIR"), "/firmware-version.rs"));

wrap_requests!(
    Node: Describe,
    Status,
    Identify,
    ReadDiagnostics,
    ReadJournal,
    OperationStatus,
    CancelOperation,
    ConfigureAuxGpio,
    PulseAuxGpio,
    Restart,
    Poll,
);

/* === Type Aliases === */

pub type NodeLabel = heapless::String<MAX_NODE_LABEL>;

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuxGpioConfigured {
    pub gpio: AuxGpioId,

    pub mode: AuxGpioMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuxGpioPulseScheduled {
    pub operation: OperationId,

    pub gpio: AuxGpioId,

    pub duration_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NodeId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SessionId(pub u16);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ChannelId(pub u8);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AuxGpioId(pub u8);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RequestId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct OperationId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EventId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirmwareVersion {
    pub major: u8,

    pub minor: u8,

    pub patch: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_type: crate::nodes::NodeType,

    pub id: NodeId,

    pub firmware: FirmwareVersion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeSummary {
    pub info: NodeInfo,

    pub silent_for_s: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeDescription {
    pub info: NodeInfo,

    pub satellites: heapless::Vec<SatelliteReport, MAX_SATELLITES>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeStatus {
    pub info: NodeInfo,

    pub auxiliary_gpio: [AuxGpioMode; AUX_GPIO_COUNT],

    pub auxiliary_gpio_access: [AuxGpioAccess; AUX_GPIO_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationSnapshot {
    pub id: OperationId,

    pub channel: ChannelId,

    pub state: OperationState,

    pub error: Option<crate::error::Error>,

    pub cleanup_error: Option<crate::error::Error>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CancelOperation {
    pub operation: OperationId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Describe;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Status;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Identify;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadDiagnostics;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadJournal {
    pub after_sequence: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationStatus {
    pub operation: OperationId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfigureAuxGpio {
    pub gpio: AuxGpioId,

    pub mode: AuxGpioMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PulseAuxGpio {
    pub gpio: AuxGpioId,

    pub duration_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Restart {
    pub bootloader: Option<Bootloader>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Poll;

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuxGpioMode {
    HighImpedance,

    DriveHigh,

    DriveLow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuxGpioAccess {
    NotAllocated,

    Reserved,

    InputOnly,

    PushPull,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuxGpioError {
    InvalidId,

    NotAllocated,

    Reserved,

    ModeNotAllowed,

    InvalidDuration,

    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperationState {
    Active,

    Completed,

    Stopped,

    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Bootloader {
    Usb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Request {
    Describe(Describe),

    Status(Status),

    Identify(Identify),

    ReadDiagnostics(ReadDiagnostics),

    ReadJournal(ReadJournal),

    OperationStatus(OperationStatus),

    CancelOperation(CancelOperation),

    ConfigureAuxGpio(ConfigureAuxGpio),

    PulseAuxGpio(PulseAuxGpio),

    Restart(Restart),

    Poll(Poll),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Description(NodeDescription),

    Status(NodeStatus),

    Identified,

    Diagnostics(NodeDiagnostics),

    Journal(JournalPage),

    OperationStatus(OperationSnapshot),

    OperationCancelled,

    AuxGpioConfigured { gpio: AuxGpioId, mode: AuxGpioMode },

    AuxGpioPulseScheduled { operation: OperationId, gpio: AuxGpioId, duration_ms: u32 },

    RestartScheduled { bootloader: Option<Bootloader> },

    Polled,
}

/* === Traits === */

/* === Implementations === */

impl From<AuxGpioConfigured> for crate::wire::Response {
    fn from(value: AuxGpioConfigured) -> Self {
        Response::AuxGpioConfigured { gpio: value.gpio, mode: value.mode }.into()
    }
}

impl From<AuxGpioPulseScheduled> for crate::wire::Response {
    fn from(value: AuxGpioPulseScheduled) -> Self {
        Response::AuxGpioPulseScheduled { operation: value.operation, gpio: value.gpio, duration_ms: value.duration_ms }.into()
    }
}

impl NodeInfo {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        self.node_type.definition().label
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl NodeId {
    pub const BROADCAST: Self = Self(0);
    pub const MIN_DEVICE: u32 = 100_000;
    pub const MAX_DEVICE: u32 = 999_999;

    #[must_use]
    pub const fn is_device(self) -> bool {
        self.0 >= Self::MIN_DEVICE && self.0 <= Self::MAX_DEVICE
    }
}

impl SessionId {
    pub const UNKNOWN: Self = Self(0);
}

impl AuxGpioId {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 >= 1 && self.0 <= AUX_GPIO_MAX_ID
    }
}

impl OperationState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Active)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_gpio_variants() {
        let configured: crate::wire::Response = AuxGpioConfigured { gpio: AuxGpioId(1), mode: AuxGpioMode::DriveHigh }.into();
        assert_eq!(configured, Response::AuxGpioConfigured { gpio: AuxGpioId(1), mode: AuxGpioMode::DriveHigh }.into());
        let scheduled: crate::wire::Response =
            AuxGpioPulseScheduled { gpio: AuxGpioId(2), duration_ms: 100, operation: OperationId(514) }.into();
        assert_eq!(scheduled, Response::AuxGpioPulseScheduled { gpio: AuxGpioId(2), duration_ms: 100, operation: OperationId(514) }.into());
    }

    #[test]
    fn only_active_terminal() {
        assert!(!OperationState::Active.is_terminal());
        assert!(OperationState::Completed.is_terminal());
        assert!(OperationState::Stopped.is_terminal());
        assert!(OperationState::Failed.is_terminal());
    }
}
