/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Stable facade errors shaped by the caller's corrective action.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::fmt;

// Third Party Imports

// Local Imports
use helios_mesh_protocol::{
    call::CallError,
    error::Error as WireError,
    node::{AuxGpioError, AuxGpioId, NodeId, OperationId},
    wire::PROTOCOL_VERSION,
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

pub type Result<T> = core::result::Result<T, Error>;

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub id: NodeId,

    pub label: String,
}

/* === Enums === */

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Device discovery and connection.
    #[error("no Helios Mesh USB device found")]
    DeviceNotFound,

    #[error("Helios Mesh USB device {0:06} was not found")]
    DeviceIdNotFound(NodeId),

    #[error(
        "multiple candidates matched and no single one was unambiguous; \
         select one explicitly: [{}]",
        candidates_list(.0)
    )]
    AmbiguousDevices(Vec<Candidate>),

    #[error("device is unavailable: {0}")]
    Device(String),

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("request timed out")]
    Timeout,

    #[error(
        "protocol version mismatch: host speaks {host}, device answered {device:?}; \
         reflash the board to matching firmware"
    )]
    ProtocolMismatch { host: u8, device: Option<u8> },

    #[error("response payload does not match the request")]
    UnexpectedResponse,

    // Local validation.
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),

    // Operation outcomes.
    #[error("operation {0:?} stopped before completion")]
    OperationStopped(OperationId),

    #[error("operation {0:?} failed in its hardware driver")]
    OperationFailed(OperationId),

    #[error(
        "node {node:06} has been silent for {silent_for_s}s according to the gateway's own \
         registry; it is unreachable"
    )]
    NodeUnreachable { node: NodeId, silent_for_s: u16 },

    // Wire rejections, mapped 1:1 from the contract's error kinds.
    #[error("request values are malformed, out of range, or violate device limits")]
    InvalidInput,

    #[error("auxiliary GPIO {} rejected: {reason:?}", gpio.0)]
    AuxGpio { gpio: AuxGpioId, reason: AuxGpioError },

    #[error("the addressed node or channel cannot perform this request")]
    Unsupported,

    #[error("the caller is not allowed to perform this request")]
    Unauthorized,

    #[error("the addressed node, channel, or operation does not exist")]
    NotFound,

    #[error("the resource is executing conflicting work; retry after it completes")]
    Busy,

    #[error("node state does not allow the request yet; change that state first")]
    Conflict,

    #[error("node writes are bound to gateway {gateway:06}; retry after its binding expires")]
    BoundElsewhere { gateway: NodeId },

    #[error("the node cannot serve the request right now; retry later")]
    Unavailable,

    #[error("a request identifier was reused with different request contents")]
    DuplicateRequest,
}

/* === Traits === */

/* === Implementations === */

impl From<&crate::UsbDevice> for Candidate {
    fn from(device: &crate::UsbDevice) -> Self {
        Self { id: device.id, label: device.label.clone() }
    }
}

impl From<&crate::node::NodeSummary> for Candidate {
    fn from(summary: &crate::node::NodeSummary) -> Self {
        Self { id: summary.info.id, label: summary.info.label().to_owned() }
    }
}

impl From<WireError> for Error {
    fn from(error: WireError) -> Self {
        match error {
            WireError::InvalidInput => Self::InvalidInput,
            WireError::AuxGpio { gpio, reason } => Self::AuxGpio { gpio, reason },
            WireError::Unsupported => Self::Unsupported,
            WireError::Unauthorized => Self::Unauthorized,
            WireError::NotFound => Self::NotFound,
            WireError::Busy => Self::Busy,
            WireError::Conflict => Self::Conflict,
            WireError::BoundElsewhere { gateway } => Self::BoundElsewhere { gateway },
            WireError::Unavailable => Self::Unavailable,
            WireError::UnsupportedVersion => Self::ProtocolMismatch { host: PROTOCOL_VERSION, device: None },
            WireError::DuplicateRequestConflict => Self::DuplicateRequest,
        }
    }
}

impl From<CallError> for Error {
    fn from(error: CallError) -> Self {
        match error {
            CallError::Rejected(error) => error.into(),
            CallError::UnexpectedResponse => Self::UnexpectedResponse,
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn candidates_list(candidates: &[Candidate]) -> String {
    use fmt::Write;

    let mut list = String::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if index > 0 {
            list.push_str(", ");
        }
        let _ = write!(list, "{:06} \"{}\"", candidate.id.0, candidate.label);
    }
    list
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_rejections_map() {
        assert!(matches!(Error::from(WireError::Busy), Error::Busy));
        assert!(matches!(Error::from(WireError::NotFound), Error::NotFound));
        assert!(matches!(
            Error::from(WireError::BoundElsewhere { gateway: NodeId(273_282) }),
            Error::BoundElsewhere { gateway: NodeId(273_282) }
        ));
        assert!(matches!(Error::from(WireError::Unavailable), Error::Unavailable));
        assert!(matches!(Error::from(WireError::UnsupportedVersion), Error::ProtocolMismatch { host: PROTOCOL_VERSION, device: None }));
    }

    #[test]
    fn call_failures_mismatch() {
        assert!(matches!(
            Error::from(CallError::Rejected(WireError::BoundElsewhere { gateway: NodeId(273_282) })),
            Error::BoundElsewhere { gateway: NodeId(273_282) }
        ));
        assert!(matches!(Error::from(CallError::UnexpectedResponse), Error::UnexpectedResponse));
    }
}
