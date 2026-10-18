/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise The Host Runtime Through In-Process Byte Streams.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm::{
    Error,
    motion::{Description, Kind},
    testing,
};
use hnm_protocol::{
    motion,
    node::{self, ChannelId as WireChannelId, NodeId, OperationId, OperationSnapshot, OperationState},
    wire::{Request, Response},
};

/* === Constants === */

const NODE: NodeId = NodeId(641_905);
const CHANNEL_1: WireChannelId = WireChannelId(1);
const CHANNEL_2: WireChannelId = WireChannelId(2);
const OPERATION: OperationId = OperationId(7);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn rotational(channel: WireChannelId) -> Description {
    Description { channel, kind: Kind::Rotational, minimum: 0, maximum: 359_999_999 }
}

fn translational(channel: WireChannelId) -> Description {
    Description { channel, kind: Kind::Translational, minimum: -1_000_000, maximum: 1_000_000 }
}

fn describe_response(_descriptions: &[Description]) -> Response {
    node::Response::Description(node::NodeDescription { info: testing::info(NODE), satellites: heapless::Vec::new() }).into()
}

#[tokio::test(start_paused = true)]
async fn axes_builds_completes() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Describe(_)) => describe_response(&[rotational(CHANNEL_2), rotational(CHANNEL_1)]),
        Request::Node(node::Request::Poll(_)) => node::Response::Polled.into(),
        Request::Node(node::Request::OperationStatus(_)) => node::Response::OperationStatus(OperationSnapshot {
            error: None,
            cleanup_error: None,
            id: OPERATION,
            channel: CHANNEL_1,
            state: OperationState::Completed,
        })
        .into(),
        Request::Motion(motion::Request::MoveRelative(_)) => motion::Response::Accepted { operation: OPERATION }.into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let guard = testing::rotator(node.clone()).axes(None).await.unwrap();
        // Advertised out of order (2 then 1); the guard normalizes to
        // channel-id order regardless of describe-response order.
        assert_eq!(guard.axes()[0].id(), CHANNEL_1);
        assert_eq!(guard.axes()[1].id(), CHANNEL_2);
        let snapshot = guard.axes()[0].turn(45.0, None, None).await.unwrap();
        assert_eq!(snapshot.state, OperationState::Completed);
    }
    let seen = responder.await.unwrap();
    assert_eq!(
        seen.iter().filter(|envelope| matches!(envelope.request, Request::Node(node::Request::Describe(_)))).count(),
        0,
        "typed axes need no capability discovery"
    );
}

#[tokio::test(start_paused = true)]
async fn axes_explicit_subset() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Describe(_)) => describe_response(&[rotational(CHANNEL_1), rotational(CHANNEL_2)]),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let guard = testing::rotator(node.clone()).axes(Some(&[CHANNEL_2])).await.unwrap();
        assert_eq!(guard.axes().len(), 1);
        assert_eq!(guard.axes()[0].id(), CHANNEL_2);
    }
    let seen = responder.await.unwrap();
    assert_eq!(seen.len(), 0, "typed channel selection needs no wire exchange");
}

#[tokio::test(start_paused = true)]
async fn stop_awaits_completion() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Describe(_)) => describe_response(&[rotational(CHANNEL_1)]),
        Request::Node(node::Request::Poll(_)) => node::Response::Polled.into(),
        Request::Node(node::Request::OperationStatus(_)) => node::Response::OperationStatus(OperationSnapshot {
            error: None,
            cleanup_error: None,
            id: OPERATION,
            channel: CHANNEL_1,
            state: OperationState::Stopped,
        })
        .into(),
        Request::Motion(motion::Request::Stop(_)) => motion::Response::Accepted { operation: OPERATION }.into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let guard = testing::rotator(node.clone()).axes(None).await.unwrap();
        let snapshot = guard.axes()[0].stop().await.unwrap();
        assert_eq!(snapshot.state, OperationState::Stopped);
    }
    responder.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn turn_move_axis() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Describe(_)) => describe_response(&[translational(CHANNEL_1)]),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let guard = testing::axes_bundle(&node, vec![translational(CHANNEL_1)]);
        assert!(matches!(guard.axes()[0].turn(10.0, None, None).await, Err(Error::InvalidArgument(_))));
        assert!(matches!(guard.axes()[0].move_to(10.0, None, None).await, Err(Error::InvalidArgument(_))));
    }
    responder.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn turn_together_axis() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Describe(_)) => describe_response(&[rotational(CHANNEL_1), rotational(CHANNEL_2)]),
        Request::Node(node::Request::Poll(_)) => node::Response::Polled.into(),
        Request::Node(node::Request::OperationStatus(status)) => node::Response::OperationStatus(OperationSnapshot {
            error: None,
            cleanup_error: None,
            id: status.operation,
            channel: CHANNEL_1,
            state: OperationState::Completed,
        })
        .into(),
        Request::Motion(motion::Request::MoveTogether(together)) => {
            let operations = together.moves.iter().map(|movement| OperationId(u32::from(movement.channel.0))).collect();
            motion::Response::AcceptedTogether { operations }.into()
        }
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let guard = testing::rotator(node.clone()).axes(None).await.unwrap();
        let snapshots = guard.turn_together(&[90.0, -90.0]).await.unwrap();
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots.iter().all(|snapshot| snapshot.state == OperationState::Completed));
    }
    responder.await.unwrap();
}

/* === Tests === */
