/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Transportable diagnostic journal entries.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::{
    motion::{Target, Timing},
    node::{ChannelId, NodeId, OperationId, RequestId},
    satellite::{SatelliteName, SatelliteState},
    wire::Event,
};

/* === Constants === */

pub const JOURNAL_PAGE_SIZE: usize = 2;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestDiagnostics {
    pub received: u32,

    pub succeeded: u32,

    pub failed: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalSummary {
    pub retained: u8,

    pub oldest_sequence: u32,

    pub latest_sequence: u32,

    pub overwritten: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeDiagnostics {
    pub node: NodeId,

    pub uptime_ms: u64,

    pub requests: RequestDiagnostics,

    pub journal: JournalSummary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub sequence: u32,

    pub timestamp_ms: u64,

    pub event: JournalEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalPage {
    pub entries: heapless::Vec<JournalEntry, JOURNAL_PAGE_SIZE>,

    pub latest_sequence: u32,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum JournalEvent {
    MotionAccepted { peer: NodeId, request_id: RequestId, channel: ChannelId, operation: OperationId, target: Target },

    CapabilityEventQueued { peer: NodeId, event: Event },

    CapabilityEventReturned { peer: NodeId, request_id: RequestId, event: Event },

    RequestRetried { peer: NodeId, request_id: RequestId, retry: u8 },

    MeshRoundTrip { peer: NodeId, request_id: RequestId, duration_ms: u32 },

    MeshRequestReceived { peer: NodeId, request_id: RequestId },

    MeshResponseSent { peer: NodeId, request_id: RequestId },

    MeshTransmit { peer: NodeId, request_id: RequestId, turnaround_ms: u32, preparation_ms: u32, transmit_ms: u32 },

    NodeEvicted { node: NodeId },

    SatelliteState { name: SatelliteName, state: SatelliteState },

    OperationCacheSaturated { node: NodeId, operation: OperationId },

    MotionStopped { channel: ChannelId, operation: OperationId, interrupted: Option<OperationId> },

    MotionCompleted { channel: ChannelId, operation: OperationId, requested: Target, actual: Target, timing: Timing },

    I2cBaudApplied { frequency_hz: u32, hcnt: u16, lcnt: u16, spklen: u8 },
}

/* === Traits === */

/* === Implementations === */

impl JournalEvent {
    #[must_use]
    pub const fn involves(self, node: NodeId) -> bool {
        match self {
            Self::MotionAccepted { peer, .. }
            | Self::CapabilityEventQueued { peer, .. }
            | Self::CapabilityEventReturned { peer, .. }
            | Self::RequestRetried { peer, .. }
            | Self::MeshRoundTrip { peer, .. }
            | Self::MeshRequestReceived { peer, .. }
            | Self::MeshResponseSent { peer, .. }
            | Self::MeshTransmit { peer, .. } => peer.0 == node.0,
            Self::NodeEvicted { node: evicted } | Self::OperationCacheSaturated { node: evicted, .. } => evicted.0 == node.0,
            Self::SatelliteState { .. } | Self::MotionStopped { .. } | Self::MotionCompleted { .. } | Self::I2cBaudApplied { .. } => false,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::MotionAccepted { .. } => "hnm.motion.accepted",
            Self::CapabilityEventQueued { .. } => "hnm.capability-event.queued",
            Self::CapabilityEventReturned { .. } => "hnm.capability-event.returned",
            Self::RequestRetried { .. } => "hnm.request.retried",
            Self::MeshRoundTrip { .. } => "hnm.mesh.round-trip",
            Self::MeshRequestReceived { .. } => "hnm.mesh.request-received",
            Self::MeshResponseSent { .. } => "hnm.mesh.response-sent",
            Self::MeshTransmit { .. } => "hnm.mesh.transmit",
            Self::NodeEvicted { .. } => "hnm.node.evicted",
            Self::SatelliteState { .. } => "hnm.satellite.state",
            Self::OperationCacheSaturated { .. } => "hnm.operation-cache.saturated",
            Self::MotionStopped { .. } => "hnm.motion.stopped",
            Self::MotionCompleted { .. } => "hnm.motion.completed",
            Self::I2cBaudApplied { .. } => "hnm.i2c.baud-applied",
        }
    }

    #[must_use]
    pub const fn peer(&self) -> Option<NodeId> {
        match self {
            Self::MotionAccepted { peer, .. }
            | Self::CapabilityEventQueued { peer, .. }
            | Self::CapabilityEventReturned { peer, .. }
            | Self::RequestRetried { peer, .. }
            | Self::MeshRoundTrip { peer, .. }
            | Self::MeshRequestReceived { peer, .. }
            | Self::MeshResponseSent { peer, .. }
            | Self::MeshTransmit { peer, .. } => Some(*peer),
            Self::NodeEvicted { node } | Self::OperationCacheSaturated { node, .. } => Some(*node),
            Self::SatelliteState { .. } | Self::MotionStopped { .. } | Self::MotionCompleted { .. } | Self::I2cBaudApplied { .. } => None,
        }
    }

    #[must_use]
    pub const fn request_id(&self) -> Option<RequestId> {
        match self {
            Self::MotionAccepted { request_id, .. }
            | Self::CapabilityEventReturned { request_id, .. }
            | Self::RequestRetried { request_id, .. }
            | Self::MeshRoundTrip { request_id, .. }
            | Self::MeshRequestReceived { request_id, .. }
            | Self::MeshResponseSent { request_id, .. }
            | Self::MeshTransmit { request_id, .. } => Some(*request_id),
            Self::CapabilityEventQueued { .. }
            | Self::NodeEvicted { .. }
            | Self::SatelliteState { .. }
            | Self::OperationCacheSaturated { .. }
            | Self::MotionStopped { .. }
            | Self::MotionCompleted { .. }
            | Self::I2cBaudApplied { .. } => None,
        }
    }

    #[must_use]
    pub const fn channel(&self) -> Option<ChannelId> {
        match self {
            Self::MotionAccepted { channel, .. } | Self::MotionStopped { channel, .. } | Self::MotionCompleted { channel, .. } => {
                Some(*channel)
            }
            _ => None,
        }
    }

    #[must_use]
    pub const fn operation(&self) -> Option<OperationId> {
        match self {
            Self::MotionAccepted { operation, .. }
            | Self::OperationCacheSaturated { operation, .. }
            | Self::MotionStopped { operation, .. }
            | Self::MotionCompleted { operation, .. } => Some(*operation),
            _ => None,
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_agree_involves() {
        let event = JournalEvent::RequestRetried { peer: NodeId(42), request_id: RequestId(9), retry: 1 };
        assert_eq!(event.name(), "hnm.request.retried");
        assert_eq!(event.peer(), Some(NodeId(42)));
        assert_eq!(event.request_id(), Some(RequestId(9)));
        assert_eq!(event.channel(), None);
        assert_eq!(event.operation(), None);
        assert!(event.involves(NodeId(42)));

        let event = JournalEvent::MotionStopped { channel: ChannelId(2), operation: OperationId(7), interrupted: None };
        assert_eq!(event.name(), "hnm.motion.stopped");
        assert_eq!(event.peer(), None);
        assert_eq!(event.request_id(), None);
        assert_eq!(event.channel(), Some(ChannelId(2)));
        assert_eq!(event.operation(), Some(OperationId(7)));

        let event = JournalEvent::SatelliteState { name: SatelliteName::Radio, state: SatelliteState::Ready };
        assert_eq!(event.name(), "hnm.satellite.state");
        assert_eq!(event.peer(), None);

        let event = JournalEvent::I2cBaudApplied { frequency_hz: 400_000, hcnt: 126, lcnt: 187, spklen: 11 };
        assert_eq!(event.name(), "hnm.i2c.baud-applied");
        assert_eq!(event.peer(), None);
        assert_eq!(event.request_id(), None);
        assert_eq!(event.channel(), None);
        assert_eq!(event.operation(), None);
        assert!(!event.involves(NodeId(42)));
    }
}
