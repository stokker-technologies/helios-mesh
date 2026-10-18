/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise The Host Runtime Through In-Process Byte Streams.

/* === Modules === */

/* === Re-Exports === */

pub use crate::transport::framing::FrameReader;
pub use crate::transport::{Connection, DEFAULT_TIMEOUT, MAX_IN_FLIGHT};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream},
    task::JoinHandle,
};

// Local Imports
use crate::NodeHandle;
use helios_mesh_protocol::{
    codec::{MAX_FRAME_SIZE, decode_request, encode_response},
    node::{FIRMWARE_VERSION, NodeId, NodeInfo, NodeStatus, RequestId},
    wire::{PROTOCOL_VERSION, Request, RequestEnvelope, Response, ResponseEnvelope},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[must_use]
pub fn typed(node: NodeHandle, kind: crate::NodeType) -> crate::ConnectedNode {
    crate::ConnectedNode::from_verified(node, kind)
}

#[must_use]
pub fn gateway(node: NodeHandle) -> crate::GatewayNode {
    crate::GatewayNode::new(node)
}

#[must_use]
pub fn sidecar(node: NodeHandle) -> crate::SidecarNode {
    let crate::ConnectedNode::Sidecar(node) = crate::ConnectedNode::from_verified(node, crate::NodeType::Sidecar) else { unreachable!() };
    node
}

#[must_use]
pub fn rotator(node: NodeHandle) -> crate::RotatorNode {
    let crate::ConnectedNode::Rotator(node) = crate::ConnectedNode::from_verified(node, crate::NodeType::Rotator) else { unreachable!() };
    node
}

#[must_use]
pub fn axes_bundle(node: &NodeHandle, channels: Vec<crate::motion::Description>) -> crate::AxesGuard {
    crate::motion::axes_bundle(node, channels)
}

#[must_use]
pub fn connected() -> (Connection, DuplexStream) {
    let (host, device) = tokio::io::duplex(4096);
    let (reader, writer) = tokio::io::split(host);
    (Connection::spawn(reader, writer), device)
}

pub async fn read_request(device: &mut DuplexStream) -> RequestEnvelope {
    let mut frame = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        device.read_exact(&mut byte).await.expect("host keeps the stream open");
        frame.push(byte[0]);
        if byte[0] == 0 {
            return decode_request(&mut frame).expect("host sends valid request frames");
        }
    }
}

pub async fn send_response(device: &mut DuplexStream, request_id: RequestId, response: Response) {
    send_versioned_response(device, PROTOCOL_VERSION, request_id, response).await;
}

pub async fn send_versioned_response(device: &mut DuplexStream, version: u8, request_id: RequestId, response: Response) {
    let envelope = ResponseEnvelope { version, request_id, response };
    let mut buffer = [0; MAX_FRAME_SIZE];
    let frame = encode_response(&envelope, &mut buffer).expect("test responses fit one frame");
    device.write_all(frame).await.expect("fake device stream stays open");
}

pub async fn send_raw(device: &mut DuplexStream, bytes: &[u8]) {
    device.write_all(bytes).await.expect("fake device stream stays open");
}

#[must_use]
pub fn status(node: NodeId) -> NodeStatus {
    NodeStatus {
        info: info(node),
        auxiliary_gpio: [helios_mesh_protocol::node::AuxGpioMode::HighImpedance; 4],
        auxiliary_gpio_access: [helios_mesh_protocol::node::AuxGpioAccess::NotAllocated; 4],
    }
}

#[must_use]
pub fn info(node: NodeId) -> NodeInfo {
    NodeInfo { node_type: helios_mesh_protocol::nodes::NodeType::Sidecar, id: node, firmware: FIRMWARE_VERSION }
}

pub fn respond(
    mut device: DuplexStream, mut script: impl FnMut(&Request) -> Response + Send + 'static,
) -> JoinHandle<Vec<RequestEnvelope>> {
    tokio::spawn(async move {
        let mut seen = Vec::new();
        loop {
            let mut frame = Vec::new();
            loop {
                let mut byte = [0_u8; 1];
                match device.read_exact(&mut byte).await {
                    Ok(_) => {}
                    Err(_) => return seen,
                }
                frame.push(byte[0]);
                if byte[0] == 0 {
                    break;
                }
            }
            let envelope = decode_request(&mut frame).expect("host sends valid request frames");
            let response = script(&envelope.request);
            send_response(&mut device, envelope.request_id, response).await;
            seen.push(envelope);
        }
    })
}

#[must_use]
pub fn node(connection: Connection, id: NodeId, via_gateway: bool) -> NodeHandle {
    NodeHandle::new(connection, id, via_gateway)
}

pub async fn select_probed(
    devices: &[crate::UsbDevice], probes: Vec<Option<(NodeHandle, Option<helios_mesh_protocol::node::NodeDescription>)>>,
) -> crate::Result<NodeHandle> {
    NodeHandle::select_probed(devices, probes).await
}

/* === Private Functions === */

/* === Tests === */
