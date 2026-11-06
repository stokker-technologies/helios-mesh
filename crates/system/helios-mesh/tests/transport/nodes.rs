/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Verify Typed Host Identity Through The In-Process Device Backend.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh::prelude::*;
use helios_mesh::testing;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

#[tokio::test(start_paused = true)]
async fn connected_enum_identity() {
    for kind in NodeType::ALL {
        let (connection, device) = testing::connected();
        let server = testing::respond(device, move |_| {
            let mut info = testing::info(NodeId(123_456));
            info.node_type = kind;
            helios_mesh_protocol::node::Response::Description(helios_mesh_protocol::node::NodeDescription {
                info,
                satellites: heapless::Vec::new(),
            })
            .into()
        });
        let handle = testing::node(connection, NodeId(123_456), false);
        let description = handle.describe().await.unwrap();
        let typed = testing::typed(handle.clone(), description.info.node_type);
        assert_eq!(typed.node_type(), kind);
        assert_eq!(typed.node().id(), NodeId(123_456));
        assert_eq!(SidecarNode::from_handle(handle.clone()).await.is_ok(), kind == NodeType::Sidecar);
        assert_eq!(RotatorNode::from_handle(handle.clone()).await.is_ok(), kind == NodeType::Rotator);
        drop(typed);
        drop(handle);
        server.await.unwrap();
    }
}

/* === Tests === */

#[tokio::test(start_paused = true)]
async fn remote_handle_correlation() {
    for leaf_count in [1, 2, 3, 7, 16] {
        let (connection, mut device) = testing::connected();
        let gateway = testing::gateway(testing::node(connection, NodeId(100_000), false));
        let mut calls = Vec::new();
        for index in 1..=leaf_count {
            let node = gateway.remote(NodeId(100_000 + index));
            calls.push(tokio::spawn(async move { SidecarNode::from_handle(node).await }));
        }

        let mut requests = Vec::new();
        for _ in 0..leaf_count {
            requests.push(testing::read_request(&mut device).await);
        }
        let mut identifiers = requests.iter().map(|request| request.request_id.0).collect::<Vec<_>>();
        identifiers.sort_unstable();
        identifiers.dedup();
        assert_eq!(identifiers.len(), requests.len());

        // Shared USB Replies Can Complete In Any Node Order.
        for request in requests.into_iter().rev() {
            let response = helios_mesh_protocol::node::Response::Description(helios_mesh_protocol::node::NodeDescription {
                info: testing::info(request.destination),
                satellites: heapless::Vec::new(),
            });
            testing::send_response(&mut device, request.request_id, response.into()).await;
        }
        for (index, call) in calls.into_iter().enumerate() {
            let sidecar = call.await.unwrap().unwrap();
            assert_eq!(sidecar.id(), NodeId(100_001 + u32::try_from(index).unwrap()));
        }

        // Typed Conversion Propagates A Radio Refusal; An Explicit Read Retry Can Succeed.
        let node = gateway.remote(NodeId(100_001));
        let failed = tokio::spawn(async move { SidecarNode::from_handle(node).await });
        let request = testing::read_request(&mut device).await;
        testing::send_response(&mut device, request.request_id, helios_mesh_protocol::error::Error::Unavailable.into()).await;
        assert!(matches!(failed.await.unwrap(), Err(Error::Unavailable)));

        let node = gateway.remote(NodeId(100_001));
        let retry = tokio::spawn(async move { SidecarNode::from_handle(node).await });
        let request = testing::read_request(&mut device).await;
        testing::send_response(
            &mut device,
            request.request_id,
            helios_mesh_protocol::node::Response::Description(helios_mesh_protocol::node::NodeDescription {
                info: testing::info(NodeId(100_001)),
                satellites: heapless::Vec::new(),
            })
            .into(),
        )
        .await;
        assert_eq!(retry.await.unwrap().unwrap().id(), NodeId(100_001));
    }
}
