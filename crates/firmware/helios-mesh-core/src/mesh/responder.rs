/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Endpoint radio loop feeding the one node dispatcher.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use defmt::warn;
use embassy_time::{Duration, Timer};
use lora_phy::{DelayNs, mod_traits::RadioKind};

// Local Imports
use super::link::{Error as RadioError, Link};
use crate::node::{Handle, Node, Origin, Submission};
use helios_mesh_protocol::{
    diagnostics::JournalEvent,
    node::{self as wire_node, NodeId, RequestId, SessionId},
    wire::{
        PROTOCOL_VERSION, Request,
        mesh::{self, Envelope, Message},
    },
};

/* === Constants === */

const RECEIVE_SLICE: Duration = Duration::from_millis(250);
const BROADCAST_REPLY_JITTER_MS: u64 = 500;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Responder<RK, DLY, R>
where
    RK: RadioKind,
    DLY: DelayNs,
    R: Node + 'static,
{
    link: Link<RK, DLY>,
    node: Handle<R>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<RK, DLY, R> Responder<RK, DLY, R>
where
    RK: RadioKind,
    DLY: DelayNs,
    R: Node + 'static,
{
    #[must_use]
    pub const fn new(link: Link<RK, DLY>, node: Handle<R>) -> Self {
        Self { link, node }
    }

    pub async fn run(mut self) -> ! {
        let node_id = self.node.node_id();
        let session_id = self.node.session_id();
        loop {
            match self.link.receive_timeout(RECEIVE_SLICE).await {
                Ok(incoming) => {
                    let Message::Request { request, acknowledged_session, acknowledged_event } = incoming.message else {
                        continue;
                    };
                    // This node has no concept of "enrolled" -- that is
                    // gateway-side registry state -- so it answers a
                    // broadcast Describe unconditionally, the same as
                    // before broadcast collision avoidance existed. The
                    // gateway's collection window absorbs a reply from an
                    // already-registered node too; that is deliberately
                    // harmless (see `absorb_broadcast_response` in
                    // `mesh::coordinator`), not filtered out here.
                    let broadcast =
                        incoming.destination == NodeId::BROADCAST && matches!(request, Request::Node(wire_node::Request::Describe(_)));
                    if !incoming.source.is_device() || (incoming.destination != node_id && !broadcast) {
                        continue;
                    }
                    if broadcast {
                        // Every unenrolled node hears the same broadcast at
                        // the same instant; wait a decorrelated slot before
                        // answering so a batch powered together doesn't
                        // transmit its replies on top of each other.
                        Timer::after(broadcast_reply_delay(node_id, session_id, incoming.request_id)).await;
                    }
                    let reply = self
                        .node
                        .mesh_request(Submission {
                            origin: Origin::Lora {
                                peer: incoming.source,
                                session: incoming.session,
                                acknowledged_session,
                                acknowledged_event,
                                broadcast,
                            },
                            version: incoming.version,
                            request_id: incoming.request_id,
                            destination: incoming.destination,
                            request,
                        })
                        .await;
                    let mut outgoing = Envelope {
                        version: PROTOCOL_VERSION,
                        source: node_id,
                        session: session_id,
                        destination: incoming.source,
                        request_id: incoming.request_id,
                        message: Message::Response { response: reply.response, event: reply.event },
                    };
                    shed_oversized_event(&mut outgoing);
                    if self.link.send(&outgoing).await.is_err() {
                        warn!("radio response failed");
                        continue;
                    }
                    self.node
                        .diagnostics()
                        .record(JournalEvent::MeshResponseSent { peer: outgoing.destination, request_id: outgoing.request_id });
                }
                Err(RadioError::Timeout) => {}
                Err(_) => {
                    warn!("radio receive failed");
                    Timer::after_millis(100).await;
                }
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn broadcast_reply_delay(node_id: NodeId, session_id: SessionId, request_id: RequestId) -> Duration {
    let mut mixed = u64::from(node_id.0).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(session_id.0).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ u64::from(request_id.0).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 33;
    mixed = mixed.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    mixed ^= mixed >> 33;
    Duration::from_millis(mixed % (BROADCAST_REPLY_JITTER_MS + 1))
}

fn shed_oversized_event(outgoing: &mut Envelope) {
    let mut probe = [0; mesh::MAX_PACKET_SIZE];
    if mesh::encode(outgoing, &mut probe).is_ok() {
        return;
    }
    if let Message::Response { event, .. } = &mut outgoing.message {
        *event = None;
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_event_unchanged() {
        let mut outgoing = envelope_with_event(helios_mesh_protocol::motion::Timing::default());
        shed_oversized_event(&mut outgoing);
        let Message::Response { event, .. } = &outgoing.message else {
            unreachable!();
        };
        assert!(event.is_some());
    }

    #[test]
    fn oversized_pair_response() {
        let wide = helios_mesh_protocol::motion::Timing {
            duration_ms: u32::MAX,
            steps: u32::MAX,
            delayed_steps: u32::MAX,
            maximum_lateness_us: u32::MAX,
            delayed_100_to_250_us: u32::MAX,
            delayed_250_to_500_us: u32::MAX,
            delayed_500_to_1000_us: u32::MAX,
            delayed_over_1000_us: u32::MAX,
            maximum_consecutive_delayed_steps: u32::MAX,
            schedule_rebases: u32::MAX,
            maximum_output_queue_us: u32::MAX,
            maximum_i2c_update_us: u32::MAX,
        };
        let mut outgoing = envelope_with_event(wide);
        let mut probe = [0; mesh::MAX_PACKET_SIZE];
        assert!(mesh::encode(&outgoing, &mut probe).is_err());
        shed_oversized_event(&mut outgoing);
        let Message::Response { event, .. } = &outgoing.message else {
            unreachable!();
        };
        assert!(event.is_none());
        assert!(mesh::encode(&outgoing, &mut probe).is_ok());
    }

    fn envelope_with_event(timing: helios_mesh_protocol::motion::Timing) -> Envelope {
        use helios_mesh_protocol::motion::{Event as MotionEvent, Target};
        use helios_mesh_protocol::node::{AuxGpioMode, ChannelId, EventId, FIRMWARE_VERSION, NodeInfo, NodeStatus, OperationId, RequestId};
        use helios_mesh_protocol::wire::{Event, EventPayload, Response};
        let status = NodeStatus {
            info: NodeInfo { node_type: helios_mesh_protocol::nodes::NodeType::Sidecar, id: NodeId(566_114), firmware: FIRMWARE_VERSION },
            auxiliary_gpio: [AuxGpioMode::HighImpedance; 4],
            auxiliary_gpio_access: [helios_mesh_protocol::node::AuxGpioAccess::NotAllocated; 4],
        };
        Envelope {
            version: PROTOCOL_VERSION,
            source: NodeId(566_114),
            session: SessionId(273),
            destination: NodeId(273_282),
            request_id: RequestId(63),
            message: Message::Response {
                response: Response::Node(wire_node::Response::Status(status)),
                event: Some(Event {
                    id: EventId(63),
                    payload: EventPayload::Motion(MotionEvent::PositionReached {
                        channel: ChannelId(2),
                        operation: OperationId(63),
                        requested: Target::Rotational { microdegrees: -45_000_000 },
                        actual: Target::Rotational { microdegrees: -45_000_000 },
                        timing,
                    }),
                }),
            },
        }
    }

    #[test]
    fn delay_stays_window() {
        for node in [0_u32, 1, 42, 566_114, u32::MAX] {
            for session in [1_u16, 2, 42, u16::MAX] {
                for request in [0_u32, 1, 42, u32::MAX] {
                    let delay = broadcast_reply_delay(NodeId(node), SessionId(session), RequestId(request));
                    assert!(delay <= Duration::from_millis(BROADCAST_REPLY_JITTER_MS));
                }
            }
        }
    }

    #[test]
    fn delay_deterministic_sweep() {
        let node = NodeId(566_114);
        let session = SessionId(273);
        let request = RequestId(9);
        assert_eq!(broadcast_reply_delay(node, session, request), broadcast_reply_delay(node, session, request));
    }

    #[test]
    fn different_nodes_window() {
        let session = SessionId(42);
        let request = RequestId(7);
        let first = broadcast_reply_delay(NodeId(100_001), session, request);
        let differs =
            [100_002_u32, 100_003, 100_004, 100_005].into_iter().any(|id| broadcast_reply_delay(NodeId(id), session, request) != first);
        assert!(differs, "sequential node ids all landed in the same slot");
    }

    #[test]
    fn reboot_changes_node() {
        let node = NodeId(100_001);
        let request = RequestId(3);
        let first = broadcast_reply_delay(node, SessionId(1), request);
        let differs = (2_u16..=9).any(|session| broadcast_reply_delay(node, SessionId(session), request) != first);
        assert!(differs, "every boot session landed in the same slot");
    }

    #[test]
    fn colliding_boot_sweeps() {
        // A small LoRa frame's airtime at SF7/BW250 is roughly 23-31ms (see
        // `mesh::coordinator::core::DISCOVERY_COLLECTION_WINDOW_MS`'s doc
        // comment, coordinator/core.rs:44-45); two replies land on top of
        // each other on the shared half-duplex link when their slots fall
        // within that span.
        const AIRTIME_MS: u64 = 31;
        // Bench-observed pairing from the live collision incident. Session
        // ids start at 1 -- `SessionId::UNKNOWN` (0) is a reserved sidecar
        // no real boot ever draws (see `session_id()` in the board crate,
        // which loops until it draws a nonzero value).
        let node_a = NodeId(566_114);
        let node_b = NodeId(471_810);
        let session_a = SessionId(1);

        // `RequestId(0)` is bit-identical to the pre-fix, two-input scheme:
        // XOR-ing in a zero-valued mix term changes nothing. So searching at
        // a fixed `RequestId(0)` is searching for a collision under the old
        // scheme -- the exact bench failure mode, where node id and session
        // id are both frozen for the boot and nothing else ever changes the
        // slot.
        let old_scheme = |node: NodeId, session: SessionId| broadcast_reply_delay(node, session, RequestId(0)).as_millis();
        let slot_a = old_scheme(node_a, session_a);
        let session_b = (1..=u16::MAX)
            .map(SessionId)
            .find(|&session_b| slot_a.abs_diff(old_scheme(node_b, session_b)) <= AIRTIME_MS)
            .expect("no colliding session id found for node_b across the u16 space");

        // Under the old scheme this pairing would collide on every single
        // sweep, forever, exactly as it did on the bench. With the fix,
        // consecutive sweeps re-roll from the request id, so the pairing
        // must not stay collided across every one of a handful of sweeps.
        let collides_every_sweep = (0_u32..8).all(|request| {
            let slot_a = broadcast_reply_delay(node_a, session_a, RequestId(request)).as_millis();
            let slot_b = broadcast_reply_delay(node_b, session_b, RequestId(request)).as_millis();
            slot_a.abs_diff(slot_b) <= AIRTIME_MS
        });
        assert!(
            !collides_every_sweep,
            "boot pairing (session_a {session_a:?}, session_b {session_b:?}) stayed collided \
             across every sweep"
        );
    }
}
