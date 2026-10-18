/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Compact `LoRa` mesh envelope and codec.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::{
    codec::CodecError,
    node::{EventId, NodeId, RequestId, SessionId},
    wire::{Event, PROTOCOL_VERSION, Request, Response},
};

/* === Constants === */

pub const MAX_PACKET_SIZE: usize = 96;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub version: u8,

    pub source: NodeId,

    pub session: SessionId,

    pub destination: NodeId,

    pub request_id: RequestId,

    pub message: Message,
}

/* === Enums === */

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Request { request: Request, acknowledged_session: SessionId, acknowledged_event: EventId },

    Response { response: Response, event: Option<Event> },
}

/* === Traits === */

/* === Implementations === */

impl Envelope {
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        self.version == PROTOCOL_VERSION
    }
}

/* === Public Functions === */

pub fn encode<'buffer>(envelope: &Envelope, buffer: &'buffer mut [u8; MAX_PACKET_SIZE]) -> Result<&'buffer [u8], CodecError> {
    postcard::to_slice(envelope, buffer).map(|encoded| &*encoded).map_err(|_| CodecError::InvalidFrame)
}

pub fn decode(bytes: &[u8]) -> Result<Envelope, CodecError> {
    postcard::from_bytes(bytes).map_err(|_| CodecError::InvalidFrame)
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::{
        diagnostics::{JOURNAL_PAGE_SIZE, JournalEntry, JournalEvent, JournalPage},
        gateway::{self, NODE_PAGE_SIZE, NodePage},
        motion::{
            Configuration, Configure, Event as MotionEvent, Limits, Mechanics, Microsteps, MoveTo, Power, Settings, StepperConfiguration,
            Target, Timing,
        },
        node::{self, AuxGpioMode, ChannelId, FIRMWARE_VERSION, NodeDescription, NodeInfo, NodeStatus, NodeSummary, OperationId},
        satellite::{self, SatelliteName, SatelliteReport, SatelliteState},
        wire::EventPayload,
    };

    const GATEWAY: NodeId = NodeId(273_282);
    const ROTATOR: NodeId = NodeId(566_114);

    fn encoded_length(envelope: &Envelope) -> usize {
        let mut buffer = [0; MAX_PACKET_SIZE];
        encode(envelope, &mut buffer).unwrap().len()
    }

    fn completed_event() -> Event {
        Event {
            id: EventId(63),
            payload: EventPayload::Motion(MotionEvent::PositionReached {
                channel: ChannelId(2),
                operation: OperationId(63),
                requested: Target::Rotational { microdegrees: -45_000_000 },
                actual: Target::Rotational { microdegrees: -45_000_000 },
                timing: Timing::default(),
            }),
        }
    }

    fn current_rotator_status() -> NodeStatus {
        NodeStatus {
            info: NodeInfo { node_type: crate::nodes::NodeType::Sidecar, id: ROTATOR, firmware: FIRMWARE_VERSION },
            auxiliary_gpio: [AuxGpioMode::HighImpedance; 4],
            auxiliary_gpio_access: [crate::node::AuxGpioAccess::NotAllocated; 4],
        }
    }

    fn current_rotator_satellites() -> heapless::Vec<SatelliteReport, { satellite::MAX_SATELLITES }> {
        heapless::Vec::from_slice(&[
            SatelliteReport { name: SatelliteName::Radio, state: SatelliteState::Ready },
            SatelliteReport { name: SatelliteName::DisplayTft, state: SatelliteState::Ready },
            SatelliteReport { name: SatelliteName::DisplayOled, state: SatelliteState::Absent },
            SatelliteReport { name: SatelliteName::MotorWing, state: SatelliteState::Absent },
        ])
        .unwrap()
    }

    fn worst_case_journal_entry() -> JournalEntry {
        JournalEntry {
            sequence: u32::MAX,
            timestamp_ms: (1_u64 << 42) - 1,
            event: JournalEvent::MeshTransmit {
                peer: NodeId(NodeId::MAX_DEVICE),
                request_id: RequestId(u32::MAX),
                turnaround_ms: u32::MAX,
                preparation_ms: u32::MAX,
                transmit_ms: u32::MAX,
            },
        }
    }

    #[test]
    fn representative_packets_budgets() {
        let motion = Envelope {
            version: PROTOCOL_VERSION,
            source: GATEWAY,
            session: SessionId(u16::MAX),
            destination: ROTATOR,
            request_id: RequestId(63),
            message: Message::Request {
                request: MoveTo {
                    channel: ChannelId(2),
                    target: Target::Rotational { microdegrees: -45_000_000 },
                    settings: Settings { speed: Some(60), acceleration: None },
                }
                .into(),
                acknowledged_session: SessionId(u16::MAX),
                acknowledged_event: EventId(62),
            },
        };
        let status = current_rotator_status();
        let pending_event = completed_event();
        let mut component_buffer = [0; MAX_PACKET_SIZE];
        let status_length = postcard::to_slice(&status, &mut component_buffer).unwrap().len();
        let event_length = postcard::to_slice(&pending_event, &mut component_buffer).unwrap().len();
        let response = Envelope {
            version: PROTOCOL_VERSION,
            source: ROTATOR,
            session: SessionId(u16::MAX),
            destination: GATEWAY,
            request_id: RequestId(63),
            message: Message::Response { response: Response::Node(node::Response::Status(status)), event: Some(pending_event) },
        };

        let motion_length = encoded_length(&motion);
        let stepper = StepperConfiguration {
            mechanics: Mechanics { full_steps: 200, microsteps: Microsteps::Eight, reversed: false },
            limits: Limits { speed: 60, acceleration: 180 },
            power: Power { moving: 25, holding: 10 },
        };
        let configuration = Envelope {
            version: PROTOCOL_VERSION,
            source: GATEWAY,
            session: SessionId(u16::MAX),
            destination: ROTATOR,
            request_id: RequestId(63),
            message: Message::Request {
                request: Configure { configuration: Configuration { steppers: heapless::Vec::from_slice(&[stepper; 2]).unwrap() } }.into(),
                acknowledged_session: SessionId(u16::MAX),
                acknowledged_event: EventId(62),
            },
        };
        let configuration_length = encoded_length(&configuration);
        let response_length = encoded_length(&response);
        let envelope_length = response_length - status_length - event_length;
        let mut buffer = [0; MAX_PACKET_SIZE];
        let encoded = encode(&response, &mut buffer).unwrap();
        assert_eq!(decode(encoded).unwrap(), response);
        std::println!(
            "motion_request_bytes={motion_length} configuration_request_bytes={configuration_length} status_bytes={status_length} event_bytes={event_length} envelope_bytes={envelope_length} status_with_event_bytes={response_length} headroom_bytes={}",
            MAX_PACKET_SIZE - response_length,
        );
        assert!(motion_length <= 32);
        assert!(configuration_length <= 64);
        assert_eq!(status_length, 26);
        assert_eq!(event_length, 27);
        assert_eq!(envelope_length, 15);
        assert_eq!(response_length, status_length + event_length + envelope_length);
        assert_eq!(response_length, 68);
        assert!(response_length <= MAX_PACKET_SIZE);
        assert_eq!(MAX_PACKET_SIZE - response_length, 28);
    }

    #[test]
    fn status_wide_risk() {
        let event = Event {
            id: EventId(u32::MAX),
            payload: EventPayload::Motion(MotionEvent::PositionReached {
                channel: ChannelId(u8::MAX),
                operation: OperationId(u32::MAX),
                requested: Target::Rotational { microdegrees: i32::MIN },
                actual: Target::Rotational { microdegrees: i32::MAX },
                timing: Timing {
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
                },
            }),
        };
        let response = Envelope {
            version: PROTOCOL_VERSION,
            source: ROTATOR,
            session: SessionId(u16::MAX),
            destination: GATEWAY,
            request_id: RequestId(u32::MAX),
            message: Message::Response { response: Response::Node(node::Response::Status(current_rotator_status())), event: Some(event) },
        };
        let mut buffer = [0_u8; 256];
        let encoded_length = postcard::to_slice(&response, &mut buffer).unwrap().len();

        std::println!(
            "wide_status_with_event_bytes={encoded_length} packet_budget_bytes={MAX_PACKET_SIZE} overflow_bytes={}",
            encoded_length - MAX_PACKET_SIZE,
        );
        assert!(encoded_length > MAX_PACKET_SIZE);
    }

    #[test]
    fn realistic_worst_packet() {
        // Reachable rather than exhaustive type-level values: identifiers at
        // sustained-session magnitudes and a completion event whose timing
        // counters carry plausible long-move values instead of the all-zero
        // default (the exhaustive u32::MAX case is
        // `status_wide_risk`
        // above, which is expected to overflow regardless). `satellites`
        // moved to `NodeDescription` (boot-static; see
        // `describe_catalogued_packet`
        // below), while node write-binding is dispatcher-local state that
        // never rides `NodeStatus`, precisely so this hot 250ms
        // collection-path pair is not taxed by either: with satellites still
        // in `NodeStatus`, this exact
        // scenario once encoded to 100 bytes, 4 over the 96-byte budget, so
        // the responder's `shed_oversized_event` dropped the completion
        // event and every affected operation fell back to the 2-second
        // `RECONCILE_AFTER_MS` reconciliation instead of resolving on this
        // exchange -- the hardware regression this test locks shut.
        let status = NodeStatus {
            info: NodeInfo { node_type: crate::nodes::NodeType::Sidecar, id: ROTATOR, firmware: FIRMWARE_VERSION },
            auxiliary_gpio: [AuxGpioMode::HighImpedance; 4],
            auxiliary_gpio_access: [crate::node::AuxGpioAccess::NotAllocated; 4],
        };
        let event = Event {
            id: EventId(5_000_000),
            payload: EventPayload::Motion(MotionEvent::PositionReached {
                channel: ChannelId(4),
                operation: OperationId(5_000_000),
                requested: Target::Rotational { microdegrees: -180_000_000 },
                actual: Target::Rotational { microdegrees: -179_999_640 },
                timing: Timing {
                    duration_ms: 60_000,
                    steps: 96_000,
                    delayed_steps: 1_200,
                    maximum_lateness_us: 800,
                    delayed_100_to_250_us: 700,
                    delayed_250_to_500_us: 350,
                    delayed_500_to_1000_us: 140,
                    delayed_over_1000_us: 10,
                    maximum_consecutive_delayed_steps: 24,
                    schedule_rebases: 3,
                    maximum_output_queue_us: 900,
                    maximum_i2c_update_us: 450,
                },
            }),
        };
        let envelope = Envelope {
            version: PROTOCOL_VERSION,
            source: ROTATOR,
            session: SessionId(u16::MAX),
            destination: GATEWAY,
            request_id: RequestId(5_000_000),
            message: Message::Response { response: Response::Node(node::Response::Status(status)), event: Some(event) },
        };
        let mut buffer = [0; 256];
        let pair_length = postcard::to_slice(&envelope, &mut buffer).unwrap().len();
        let Message::Response { response, .. } = envelope.message.clone() else {
            unreachable!();
        };
        let status_alone = Envelope { message: Message::Response { response, event: None }, ..envelope };
        let status_length = encoded_length(&status_alone);
        std::println!(
            "realistic_pair_bytes={pair_length} realistic_status_bytes={status_length} pair_headroom_bytes={}",
            MAX_PACKET_SIZE - pair_length,
        );
        // The response must always fit on its own: the responder sheds the
        // piggybacked event when the pair exceeds the budget (see
        // `shed_oversized_event` in `helios-mesh-core::mesh::responder`), so the
        // event is opportunistic and the response is the invariant.
        assert!(status_length <= MAX_PACKET_SIZE);
        // The pair must fit, with the shortfall satellites caused (this
        // exact scenario encoded to 100 bytes, 4 over budget, while
        // satellites still lived on `NodeStatus`) more than reclaimed --
        // not merely broken even on a single knife-edge byte.
        assert!(pair_length <= MAX_PACKET_SIZE);
        assert!(MAX_PACKET_SIZE - pair_length >= 4, "pair headroom did not recover the satellites-era shortfall");
    }

    #[test]
    fn describe_catalogued_packet() {
        // Satellites are boot-static (`helios-mesh-core::boot::BootReport` is sealed
        // once and never revised), so they ride the rarely-repeated Describe
        // exchange rather than the 250ms Status collection cadence. Every
        // catalogued satellite and both motion capabilities are represented,
        // the same shape a real rotator's enrollment response carries.
        let description = NodeDescription {
            info: NodeInfo { node_type: crate::nodes::NodeType::Sidecar, id: ROTATOR, firmware: FIRMWARE_VERSION },
            satellites: current_rotator_satellites(),
        };
        let envelope = Envelope {
            version: PROTOCOL_VERSION,
            source: ROTATOR,
            session: SessionId(u16::MAX),
            destination: GATEWAY,
            request_id: RequestId(63),
            message: Message::Response { response: Response::Node(node::Response::Description(description)), event: None },
        };
        let length = encoded_length(&envelope);
        std::println!("describe_with_satellites_bytes={length} packet_budget_bytes={MAX_PACKET_SIZE}");
        assert!(length <= MAX_PACKET_SIZE);
    }

    #[test]
    fn full_journal_packet() {
        let entry = worst_case_journal_entry();
        let mut entries = heapless::Vec::new();
        for _ in 0..JOURNAL_PAGE_SIZE {
            entries.push(entry).unwrap();
        }
        let envelope = Envelope {
            version: PROTOCOL_VERSION,
            source: NodeId(NodeId::MAX_DEVICE),
            session: SessionId(u16::MAX),
            destination: NodeId(NodeId::MAX_DEVICE),
            request_id: RequestId(u32::MAX),
            message: Message::Response {
                response: Response::Node(node::Response::Journal(JournalPage { entries, latest_sequence: u32::MAX })),
                event: None,
            },
        };
        let page_length = encoded_length(&envelope);
        let mut buffer = [0; MAX_PACKET_SIZE];
        let entry_length = postcard::to_slice(&entry, &mut buffer).unwrap().len();
        std::println!("journal_page_bytes={page_length} journal_entry_bytes={entry_length}");
        assert!(page_length <= MAX_PACKET_SIZE);
        assert!(page_length + entry_length > MAX_PACKET_SIZE, "JOURNAL_PAGE_SIZE can be raised");
    }

    #[test]
    fn i2c_baud_comfortably() {
        let entry = JournalEntry {
            sequence: u32::MAX,
            timestamp_ms: (1_u64 << 42) - 1,
            event: JournalEvent::I2cBaudApplied { frequency_hz: u32::MAX, hcnt: u16::MAX, lcnt: u16::MAX, spklen: u8::MAX },
        };
        let mut buffer = [0; MAX_PACKET_SIZE];
        let entry_length = postcard::to_slice(&entry, &mut buffer).unwrap().len();
        let worst_case_length = {
            let mut buffer = [0; MAX_PACKET_SIZE];
            postcard::to_slice(&worst_case_journal_entry(), &mut buffer).unwrap().len()
        };
        std::println!("i2c_baud_applied_bytes={entry_length}");
        assert!(entry_length <= worst_case_length);
    }

    #[test]
    fn motion_completed_risk() {
        let entry = JournalEntry {
            sequence: u32::MAX,
            timestamp_ms: (1_u64 << 42) - 1,
            event: JournalEvent::MotionCompleted {
                channel: ChannelId(u8::MAX),
                operation: OperationId(u32::MAX),
                requested: Target::Rotational { microdegrees: i32::MIN },
                actual: Target::Rotational { microdegrees: i32::MAX },
                timing: Timing {
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
                },
            },
        };
        let mut buffer = [0_u8; 256];
        let entry_length = postcard::to_slice(&entry, &mut buffer).unwrap().len();
        std::println!(
            "motion_completed_worst_case_entry_bytes={entry_length} \
             packet_budget_bytes={MAX_PACKET_SIZE}"
        );
        assert!(entry_length > MAX_PACKET_SIZE / JOURNAL_PAGE_SIZE);
    }

    #[test]
    fn motion_completed_budget() {
        let entry = JournalEntry {
            sequence: 5_000_000,
            timestamp_ms: 3_600_000,
            event: JournalEvent::MotionCompleted {
                channel: ChannelId(2),
                operation: OperationId(5_000_000),
                requested: Target::Rotational { microdegrees: 10_000_000 },
                actual: Target::Rotational { microdegrees: 9_999_640 },
                timing: Timing {
                    duration_ms: 250,
                    steps: 400,
                    delayed_steps: 0,
                    maximum_lateness_us: 80,
                    delayed_100_to_250_us: 0,
                    delayed_250_to_500_us: 0,
                    delayed_500_to_1000_us: 0,
                    delayed_over_1000_us: 0,
                    maximum_consecutive_delayed_steps: 0,
                    schedule_rebases: 0,
                    maximum_output_queue_us: 0,
                    maximum_i2c_update_us: 220,
                },
            },
        };
        let mut buffer = [0_u8; 256];
        let entry_length = postcard::to_slice(&entry, &mut buffer).unwrap().len();
        std::println!(
            "motion_completed_realistic_entry_bytes={entry_length} \
             packet_budget_bytes={MAX_PACKET_SIZE}"
        );
        // A single realistic entry, not just a per-page share of the budget:
        // over a mesh-relayed `ReadJournal` this still has to coexist with
        // the envelope and (when `JOURNAL_PAGE_SIZE` > 1) a sibling entry,
        // so comfortable headroom below the whole packet budget, not merely
        // under it, is the honest bar.
        assert!(entry_length < MAX_PACKET_SIZE / 2);
    }

    fn rotator_summary(id: u32) -> NodeSummary {
        NodeSummary {
            info: NodeInfo { node_type: crate::nodes::NodeType::Sidecar, id: NodeId(id), firmware: FIRMWARE_VERSION },
            silent_for_s: 0,
        }
    }

    fn node_page_envelope(page: NodePage) -> Envelope {
        Envelope {
            version: PROTOCOL_VERSION,
            source: GATEWAY,
            session: SessionId(u16::MAX),
            destination: ROTATOR,
            request_id: RequestId(63),
            message: Message::Response { response: Response::Gateway(gateway::Response::Nodes(page)), event: None },
        }
    }

    #[test]
    fn representative_node_capabilities() {
        // `NodePage` entries carry capabilities as of protocol v8 (see
        // `helios-mesh-core::mesh::coordinator::core::GatewayCore::nodes`, served
        // straight from each registered node's cached `NodeDescription` with
        // no live query -- `scan()` is zero-air). Unlike every other
        // response measured in this module, `NodePage` never actually
        // crosses the mesh: `helios-mesh-core::node::origin_allows` sets
        // `mesh: false` for every `wire::Request::Gateway` variant --
        // test-locked by
        // `helios-mesh-core::node::tests::dispatcher_rejects_only_origin_specific_requests`
        // -- so this response is always USB-framed, bounded by
        // `codec::MAX_FRAME_SIZE` (256B; see
        // `codec::tests::realistic_node_page_fits_one_usb_frame`), never by
        // this module's 96B `MAX_PACKET_SIZE`. Measured here anyway, as a
        // record of what a future change that *did* relay rosters over LoRa
        // would need to reckon with: a full page of today's fleet's
        // realistic worst case (two-channel rotators, the only capability-
        // bearing node type this mesh has) does not fit the LoRa budget, so
        // `NODE_PAGE_SIZE` stays at its USB-framed value of 4 rather than
        // shrinking for a constraint that does not apply.
        let mut nodes = heapless::Vec::new();
        for offset in 0..u32::try_from(NODE_PAGE_SIZE).unwrap() {
            nodes.push(rotator_summary(200_000 + offset)).unwrap();
        }
        let envelope = node_page_envelope(NodePage { nodes, next_after: Some(NodeId(999_999)) });
        let mut buffer = [0_u8; 256];
        let length = postcard::to_slice(&envelope, &mut buffer).unwrap().len();
        std::println!("realistic_node_page_bytes={length} mesh_packet_budget_bytes={MAX_PACKET_SIZE} node_page_size={NODE_PAGE_SIZE}");
        assert!(length <= MAX_PACKET_SIZE);
    }
}
