/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Main Rust facade for discovering and controlling Helios Mesh nodes.

#![cfg_attr(not(feature = "host"), no_std)]

/* === Modules === */

pub mod prelude;

#[cfg(all(feature = "host", any(test, feature = "testing")))]
pub mod testing;

#[cfg(feature = "host")]
mod error;

#[cfg(feature = "host")]
pub mod gateway;

#[cfg(feature = "host")]
pub mod journal;

#[cfg(feature = "host")]
pub mod motion;

#[cfg(feature = "host")]
pub mod node;

#[cfg(feature = "host")]
mod transport;

#[cfg(feature = "host")]
pub mod nodes;

#[cfg(feature = "firmware")]
pub mod __private {
    #[cfg(target_arch = "arm")]
    pub use embassy_executor;
    pub use helios_mesh_core::jobs::ChannelStorage;
    #[cfg(target_arch = "arm")]
    pub use static_cell::StaticCell;
}

/* === Re-Exports === */

pub use helios_mesh_macros::{jobs, node, node_catalog, route};
pub use helios_mesh_protocol::error::Error as RequestError;
pub use helios_mesh_protocol::node::{AuxGpioConfigured, AuxGpioId, AuxGpioMode, AuxGpioPulseScheduled, ConfigureAuxGpio, PulseAuxGpio};
pub use helios_mesh_protocol::nodes::{NodeDefinition, NodeType};
pub use helios_mesh_protocol::wire::{Request, Response};

#[cfg(feature = "host")]
pub use error::{Candidate, Error, Result};

#[cfg(feature = "host")]
pub use gateway::{GatewayNode, NODE_PAGE_SIZE, NodePage, NodeTarget};

#[cfg(feature = "host")]
pub use motion::{AxesGuard, Axis, StepperConfig};

#[cfg(feature = "host")]
pub use node::{DeviceSelector, NodeHandle, Operation};

#[cfg(feature = "host")]
pub use transport::{UsbDevice, usb_devices};

#[cfg(feature = "host")]
pub use nodes::{ConnectedNode, DiscoveredNode, RotatorNode, SidecarNode, discover, discover_type};

#[cfg(feature = "firmware")]
pub use helios_mesh_core::node::{Ctx as Context, Node, Outcome};

#[cfg(feature = "firmware")]
pub use helios_mesh_core::jobs::{
    Channel, ChannelRef, Job, JobContext, JobError, JobOperation, JobOutcome, JobStatus, MAX_JOB_CHILDREN, ResourceGuard,
};

#[cfg(not(feature = "host"))]
pub use helios_mesh_protocol::error::Error;

#[cfg(all(feature = "firmware", target_arch = "arm"))]
pub use helios_mesh_board_feather_rp2040::{Auxiliary, AuxiliaryPin, Board, Drive, Hardware, HardwareError, ValidatedHardware, Wing};

#[cfg(all(feature = "firmware", target_arch = "arm"))]
pub use helios_mesh_board_feather_rp2040::wings::{Motor, Oled, TftV2};

#[cfg(feature = "firmware")]
pub use helios_mesh_core::jobs::Duration;
pub use helios_mesh_protocol::motion::{
    Accepted as MotionAccepted, AcceptedTogether as MotionAcceptedTogether, Configure as ConfigureMotion, Configured as MotionConfigured,
    Delta as MotionDelta, MoveRelative, MoveTo, MoveTogether, Response as MotionResponse, SetHome, Settings as MotionSettings,
    StepperConfiguration, Stop, Target as MotionTarget,
};
pub use helios_mesh_protocol::node::Response as NodeResponse;
pub use helios_mesh_protocol::node::{
    AuxGpioAccess, AuxGpioError, Bootloader, FirmwareVersion, NodeDescription, NodeId, NodeInfo, NodeStatus, NodeSummary,
};
pub use helios_mesh_protocol::node::{ChannelId, OperationId, OperationSnapshot, OperationState};

#[cfg(feature = "firmware")]
pub use helios_mesh_core::{boot::BootReport, diagnostics::Diagnostics};

#[cfg(feature = "firmware")]
pub use helios_mesh_drivers::motor::{
    Completions as MotorCompletions, Config as MotorConfig, Handle as MotorHandle, Move as MotorMove, Outcome as MotorOutcome,
    nearest_heading_target,
};
pub use helios_mesh_protocol::diagnostics::JournalEvent;
pub use helios_mesh_protocol::satellite::SatelliteName;

#[cfg(feature = "host")]
pub use helios_mesh_protocol::call::{Call, CallError};

#[cfg(feature = "firmware")]
pub use helios_mesh_core::node::Origin;

#[cfg(feature = "firmware")]
pub use helios_mesh_drivers::motor;

pub use helios_mesh_protocol::diagnostics::{
    JOURNAL_PAGE_SIZE, JournalEntry, JournalPage, JournalSummary, NodeDiagnostics, RequestDiagnostics,
};
#[cfg(not(feature = "host"))]
pub use helios_mesh_protocol::motion;
pub use helios_mesh_protocol::motion::{
    Configuration as MotionConfiguration, Description as MotionDescription, Kind as MotionKind, Limits, Mechanics, Microsteps, Power,
    Timing as MotionTiming,
};
pub use helios_mesh_protocol::node::{
    AUX_GPIO_COUNT, CancelOperation, Describe, Identify, MAX_CHANNELS, MAX_NODE_LABEL, NodeLabel, OperationStatus, Poll, ReadDiagnostics,
    ReadJournal, RequestId, Restart, SessionId, Status,
};
pub use helios_mesh_protocol::satellite::{MAX_SATELLITES, SatelliteReport, SatelliteState};

#[cfg(feature = "host")]
pub use journal::{
    Cursor as JournalCursor, Header as JournalHeader, Kind as JournalKind, Metadata as JournalMetadata, RECORD_VERSION,
    Record as JournalRecord, SCHEMA_VERSION, event_name, now_us,
};

#[cfg(feature = "host")]
pub use motion::timing_of;

#[cfg(feature = "firmware")]
pub use helios_mesh_core::node::{StatusRow, StatusRows, StatusValue};

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

#[cfg(not(feature = "host"))]
pub type Result<T> = core::result::Result<T, Error>;

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
