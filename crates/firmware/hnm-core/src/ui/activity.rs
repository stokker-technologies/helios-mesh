/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Command-agnostic request activity observed at the service boundary.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::node::Origin;
use hnm_protocol::node::{NodeId, RequestId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestActivity {
    pub request_id: RequestId,

    pub source: Origin,

    pub target: Option<NodeId>,

    pub state: ActivityState,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivityState {
    Pending,

    Succeeded,

    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivityDirection {
    LocalUsb,

    Incoming(NodeId),

    Outgoing(NodeId),
}

/* === Traits === */

/* === Implementations === */

impl RequestActivity {
    #[must_use]
    pub const fn observed(origin: Origin, request_id: RequestId, destination: NodeId) -> Self {
        Self { request_id, source: origin, target: Some(destination), state: ActivityState::Pending }
    }

    pub fn complete(&mut self, origin: Origin, request_id: RequestId, succeeded: bool) {
        if self.request_id == request_id && self.source == origin {
            self.state = if succeeded { ActivityState::Succeeded } else { ActivityState::Failed };
        }
    }

    #[must_use]
    pub const fn peer(self, local: NodeId) -> Option<NodeId> {
        if let Some(target) = self.target
            && target.0 != local.0
            && target.is_device()
        {
            return Some(target);
        }
        match self.source {
            Origin::Usb => None,
            Origin::Lora { peer, .. } => Some(peer),
        }
    }

    #[must_use]
    pub fn direction_from(self, viewpoint: NodeId, observer: NodeId) -> Option<ActivityDirection> {
        let target = self.target.filter(|target| target.is_device());
        match self.source {
            Origin::Usb if viewpoint == observer => match target {
                Some(target) if target != viewpoint => Some(ActivityDirection::Outgoing(target)),
                _ => Some(ActivityDirection::LocalUsb),
            },
            Origin::Usb if target == Some(viewpoint) => Some(ActivityDirection::Incoming(observer)),
            Origin::Lora { peer, .. } if peer == viewpoint => target.filter(|target| *target != viewpoint).map(ActivityDirection::Outgoing),
            Origin::Lora { peer, .. } if target == Some(viewpoint) || viewpoint == observer => Some(ActivityDirection::Incoming(peer)),
            Origin::Usb | Origin::Lora { .. } => None,
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::node::{EventId, NodeId, RequestId, SessionId};

    use super::*;

    const fn lora() -> Origin {
        Origin::Lora {
            peer: NodeId(273_282),
            session: SessionId(3),
            acknowledged_session: SessionId(3),
            acknowledged_event: EventId(0),
            broadcast: false,
        }
    }

    #[test]
    fn observation_retains_destination() {
        let activity = RequestActivity::observed(Origin::Usb, RequestId(17), NodeId(566_114));

        assert_eq!(activity.request_id, RequestId(17));
        assert_eq!(activity.target, Some(NodeId(566_114)));
        assert_eq!(activity.state, ActivityState::Pending);
    }

    #[test]
    fn only_matching_request() {
        let mut activity = RequestActivity::observed(lora(), RequestId(19), NodeId(566_114));

        activity.complete(lora(), RequestId(18), false);
        assert_eq!(activity.state, ActivityState::Pending);
        activity.complete(Origin::Usb, RequestId(19), false);
        assert_eq!(activity.state, ActivityState::Pending);
        activity.complete(lora(), RequestId(19), true);
        assert_eq!(activity.state, ActivityState::Succeeded);
        assert_eq!(activity.peer(NodeId(566_114)), Some(NodeId(273_282)));
    }

    #[test]
    fn forwarded_activity_page() {
        let activity = RequestActivity::observed(Origin::Usb, RequestId(20), NodeId(566_114));

        assert_eq!(activity.direction_from(NodeId(273_282), NodeId(273_282)), Some(ActivityDirection::Outgoing(NodeId(566_114))));
        assert_eq!(activity.direction_from(NodeId(566_114), NodeId(273_282)), Some(ActivityDirection::Incoming(NodeId(273_282))));
    }
}
