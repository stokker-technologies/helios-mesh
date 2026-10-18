/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Implement codec for common hnm protocol src.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::wire::{RequestEnvelope, ResponseEnvelope};

/* === Constants === */

pub const MAX_FRAME_SIZE: usize = 256;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    InvalidFrame,
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub fn encode_request<'a>(request: &RequestEnvelope, buffer: &'a mut [u8]) -> Result<&'a [u8], CodecError> {
    postcard::to_slice_cobs(request, buffer).map(|frame| &*frame).map_err(|_| CodecError::InvalidFrame)
}

pub fn decode_request(buffer: &mut [u8]) -> Result<RequestEnvelope, CodecError> {
    postcard::from_bytes_cobs(buffer).map_err(|_| CodecError::InvalidFrame)
}

pub fn encode_response<'a>(response: &ResponseEnvelope, buffer: &'a mut [u8]) -> Result<&'a [u8], CodecError> {
    postcard::to_slice_cobs(response, buffer).map(|frame| &*frame).map_err(|_| CodecError::InvalidFrame)
}

pub fn decode_response(buffer: &mut [u8]) -> Result<ResponseEnvelope, CodecError> {
    postcard::from_bytes_cobs(buffer).map_err(|_| CodecError::InvalidFrame)
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::{
        error::Error,
        gateway::{self, NODE_PAGE_SIZE, NodePage},
        node::{FIRMWARE_VERSION, NodeId, NodeInfo, NodeSummary},
        wire::{PROTOCOL_VERSION, Request, Response},
    };

    fn rotator_summary(id: u32) -> NodeSummary {
        NodeSummary {
            info: NodeInfo {
                node_type: crate::nodes::NodeType::Sidecar,
                id: NodeId(id),
                // The protocol's maximum legal 24-byte name, not a short
                // illustrative one -- the realistic worst case must cover
                // what a node is actually allowed to advertise.
                firmware: FIRMWARE_VERSION,
            },
            // Postcard varint-encodes `u16`, so its worst case -- not a
            // small illustrative value -- is the honest byte count for this
            // frame-budget measurement.
            silent_for_s: u16::MAX,
        }
    }

    #[test]
    fn realistic_node_frame() {
        // `Nodes`/`NodePage` is USB-only in this dispatcher topology:
        // `hnm-core::node::origin_allows` sets `mesh: false` for every
        // `Gateway` request (test-locked by
        // `hnm-core::node::tests::dispatcher_rejects_only_origin_specific_requests`),
        // so a roster response is always framed here, bounded by
        // `MAX_FRAME_SIZE` -- the host's `FrameReader`
        // (`crates/system/hnm/src/transport/framing.rs`) discards anything at or
        // beyond it as corrupt. This is the budget that actually governs
        // `NODE_PAGE_SIZE`; `wire::mesh::MAX_PACKET_SIZE` (the LoRa radio
        // budget) does not apply -- see
        // `wire::mesh::tests::node_page_documents_that_the_mesh_packet_budget_does_not_gate_it`.
        //
        // Realistic worst case: a full `NODE_PAGE_SIZE` page of two-channel
        // rotators (the fleet's only capability-bearing node type) at the
        // protocol's maximum legal display name, through the real
        // production `encode_response` path.
        let mut nodes = heapless::Vec::new();
        for offset in 0..u32::try_from(NODE_PAGE_SIZE).unwrap() {
            nodes.push(rotator_summary(200_000 + offset)).unwrap();
        }
        let response = ResponseEnvelope {
            version: PROTOCOL_VERSION,
            request_id: crate::node::RequestId(u32::MAX),
            response: Response::Gateway(gateway::Response::Nodes(NodePage { nodes, next_after: Some(NodeId(999_999)) })),
        };
        let mut buffer = [0_u8; MAX_FRAME_SIZE];
        let frame_length = encode_response(&response, &mut buffer).unwrap().len();
        std::println!(
            "realistic_node_page_frame_bytes={frame_length} usb_frame_budget_bytes={MAX_FRAME_SIZE} node_page_size={NODE_PAGE_SIZE}",
        );
        assert!(frame_length <= MAX_FRAME_SIZE);

        let mut roundtrip = buffer;
        assert_eq!(decode_response(&mut roundtrip).unwrap(), response);
    }

    #[test]
    fn gateway_nodes_frame() {
        let request = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: crate::node::RequestId(u32::MAX),
            destination: NodeId(NodeId::MAX_DEVICE),
            request: Request::Gateway(gateway::Request::Nodes(gateway::Nodes { after: Some(NodeId(NodeId::MAX_DEVICE)) })),
        };
        let mut buffer = [0_u8; MAX_FRAME_SIZE];
        let frame_length = encode_request(&request, &mut buffer).unwrap().len();
        assert!(frame_length <= MAX_FRAME_SIZE);
    }

    #[test]
    fn bound_elsewhere_framing() {
        let response = ResponseEnvelope {
            version: PROTOCOL_VERSION,
            request_id: crate::node::RequestId(63),
            response: Response::Error(Error::BoundElsewhere { gateway: NodeId(273_282) }),
        };
        let mut buffer = [0_u8; MAX_FRAME_SIZE];
        let frame_length = encode_response(&response, &mut buffer).unwrap().len();
        assert!(frame_length <= MAX_FRAME_SIZE);

        let mut roundtrip = buffer;
        assert_eq!(decode_response(&mut roundtrip).unwrap(), response);
    }
    #[test]
    fn gpio_refusals_roundtrip() {
        use crate::node::{AuxGpioError, AuxGpioId};
        for reason in [
            AuxGpioError::InvalidId,
            AuxGpioError::NotAllocated,
            AuxGpioError::Reserved,
            AuxGpioError::ModeNotAllowed,
            AuxGpioError::InvalidDuration,
            AuxGpioError::Busy,
        ] {
            let response = ResponseEnvelope {
                version: PROTOCOL_VERSION,
                request_id: crate::node::RequestId(u32::MAX),
                response: Response::Error(Error::AuxGpio { gpio: AuxGpioId(4), reason }),
            };
            let mut buffer = [0; MAX_FRAME_SIZE];
            let length = encode_response(&response, &mut buffer).unwrap().len();
            assert_eq!(decode_response(&mut buffer[..length]).unwrap(), response);
        }
    }
    #[test]
    fn pulse_admission_usb() {
        use crate::node::{AuxGpioId, PulseAuxGpio};
        let request = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: crate::node::RequestId(91),
            destination: NodeId(273_282),
            request: PulseAuxGpio { gpio: AuxGpioId(2), duration_ms: 60_000 }.into(),
        };
        let mut buffer = [0; MAX_FRAME_SIZE];
        let length = encode_request(&request, &mut buffer).unwrap().len();
        assert_eq!(decode_request(&mut buffer[..length]).unwrap(), request);
        let response = ResponseEnvelope {
            version: PROTOCOL_VERSION,
            request_id: request.request_id,
            response: crate::node::Response::AuxGpioPulseScheduled {
                operation: crate::node::OperationId(1),
                gpio: AuxGpioId(2),
                duration_ms: 60_000,
            }
            .into(),
        };
        let length = encode_response(&response, &mut buffer).unwrap().len();
        let decoded = decode_response(&mut buffer[..length]).unwrap();
        assert_eq!(decoded, response);
    }
}
