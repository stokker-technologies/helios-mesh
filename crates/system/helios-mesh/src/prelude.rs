/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Convenient imports for Helios Mesh host applications and firmware.

/* === Modules === */

/* === Re-Exports === */

pub use crate::{
    AuxGpioAccess, AuxGpioConfigured, AuxGpioError, AuxGpioId, AuxGpioMode, AuxGpioPulseScheduled, Bootloader, CancelOperation, ChannelId,
    ConfigureAuxGpio, ConfigureMotion, Describe, Error, FirmwareVersion, Identify, JournalEntry, JournalEvent, JournalPage, JournalSummary,
    Limits, Mechanics, Microsteps, MotionAccepted, MotionAcceptedTogether, MotionConfiguration, MotionConfigured, MotionDelta,
    MotionDescription, MotionKind, MotionResponse, MotionSettings, MotionTarget, MotionTiming, MoveRelative, MoveTo, MoveTogether,
    NodeDefinition, NodeDescription, NodeDiagnostics, NodeId, NodeInfo, NodeLabel, NodeResponse, NodeStatus, NodeSummary, NodeType,
    OperationId, OperationSnapshot, OperationState, OperationStatus, Poll, Power, PulseAuxGpio, ReadDiagnostics, ReadJournal, Request,
    RequestDiagnostics, RequestError, RequestId, Response, Restart, Result, SatelliteName, SatelliteReport, SatelliteState, SessionId,
    SetHome, Status, StepperConfiguration, Stop, jobs, node, node_catalog, route,
};

#[cfg(feature = "host")]
pub use crate::{
    AxesGuard, Axis, Call, CallError, Candidate, ConnectedNode, DeviceSelector, DiscoveredNode, GatewayNode, NodeHandle, NodePage,
    NodeTarget, Operation, RotatorNode, SidecarNode, StepperConfig, UsbDevice, discover, discover_type, timing_of, usb_devices,
};

#[cfg(feature = "firmware")]
pub use crate::{
    BootReport, Channel, ChannelRef, Context, Diagnostics, Duration, Job, JobContext, JobError, JobOperation, JobOutcome, JobStatus,
    MotorCompletions, MotorConfig, MotorHandle, MotorMove, MotorOutcome, Node, Origin, Outcome, ResourceGuard, StatusRow, StatusRows,
    StatusValue, motor, nearest_heading_target,
};

#[cfg(all(feature = "firmware", target_arch = "arm"))]
pub use crate::{Auxiliary, AuxiliaryPin, Board, Drive, Hardware, HardwareError, Motor, Oled, TftV2, ValidatedHardware, Wing};

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
    use super::*;

    #[test]
    fn shared_identity() {
        let id: crate::NodeId = NodeId(42);
        assert_eq!(id, NodeId(42));
    }

    #[cfg(feature = "host")]
    #[test]
    fn host_identity() {
        let _: Option<GatewayNode> = None::<crate::GatewayNode>;
        let _: Option<DeviceSelector> = None::<crate::DeviceSelector>;
    }

    #[cfg(feature = "firmware")]
    #[test]
    fn firmware_identity() {
        let _: Option<MotorHandle> = None::<crate::MotorHandle>;
        let _: Option<JobStatus> = None::<crate::JobStatus>;
    }
}
