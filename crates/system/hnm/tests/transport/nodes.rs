/* ***
 *
 * Helios Node Mesh
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
use hnm::testing;
use hnm::{NodeId, NodeType, RotatorNode, SidecarNode};

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
            hnm_protocol::node::Response::Description(hnm_protocol::node::NodeDescription { info, satellites: heapless::Vec::new() }).into()
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
