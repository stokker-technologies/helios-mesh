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

/* === Imports === */

// Standard Library Imports
// Third Party Imports
use std::{
    future::Future,
    task::{Context, Waker},
};

// Local Imports
use helios_mesh::{
    Error,
    testing::{self, DEFAULT_TIMEOUT, MAX_IN_FLIGHT},
};
use helios_mesh_protocol::{
    node::{self, NodeId, NodeStatus},
    wire::{PROTOCOL_VERSION, Request},
};

/* === Constants === */

const NODE: NodeId = NodeId(273_282);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

#[tokio::test(start_paused = true)]
async fn out_order_requests() {
    let (connection, mut device) = testing::connected();
    let identify = {
        let connection = connection.clone();
        tokio::spawn(async move { connection.call(NODE, DEFAULT_TIMEOUT, node::Identify).await })
    };
    let status = {
        let connection = connection.clone();
        tokio::spawn(async move { connection.call(NODE, DEFAULT_TIMEOUT, node::Status).await })
    };

    let first = testing::read_request(&mut device).await;
    let second = testing::read_request(&mut device).await;
    // Answer in reverse arrival order.
    for envelope in [&second, &first] {
        let response = match &envelope.request {
            Request::Node(node::Request::Identify(_)) => node::Response::Identified.into(),
            Request::Node(node::Request::Status(_)) => node::Response::Status(testing::status(NODE)).into(),
            other => panic!("unexpected request: {other:?}"),
        };
        testing::send_response(&mut device, envelope.request_id, response).await;
    }

    identify.await.unwrap().unwrap();
    let status: NodeStatus = status.await.unwrap().unwrap();
    assert_eq!(status.info.id, NODE);
}

#[tokio::test(start_paused = true)]
async fn foreign_response_mismatch() {
    let (connection, mut device) = testing::connected();
    let call = tokio::spawn(async move { connection.call(NODE, DEFAULT_TIMEOUT, node::Identify).await });
    let request = testing::read_request(&mut device).await;
    testing::send_versioned_response(&mut device, 1, request.request_id, node::Response::Identified.into()).await;
    assert!(matches!(call.await.unwrap(), Err(Error::ProtocolMismatch { host: PROTOCOL_VERSION, device: Some(1) })));
}

#[tokio::test(start_paused = true)]
async fn corrupt_frame_survives() {
    let (connection, mut device) = testing::connected();
    let call = tokio::spawn(async move { connection.call(NODE, DEFAULT_TIMEOUT, node::Identify).await });
    let request = testing::read_request(&mut device).await;
    // Garbage that decodes as neither a response nor an envelope head.
    testing::send_raw(&mut device, &[0xFF, 0xFE, 0xFD, 0]).await;
    testing::send_response(&mut device, request.request_id, node::Response::Identified.into()).await;
    call.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn flight_requests_bounded() {
    let (connection, mut device) = testing::connected();
    let mut calls = Vec::new();
    for _ in 0..=MAX_IN_FLIGHT {
        let connection = connection.clone();
        calls.push(tokio::spawn(async move { connection.call(NODE, DEFAULT_TIMEOUT, node::Identify).await }));
    }

    let mut sent = Vec::new();
    for _ in 0..MAX_IN_FLIGHT {
        sent.push(testing::read_request(&mut device).await);
    }
    // The extra request must wait until one in-flight slot frees.
    {
        let mut waiting = std::pin::pin!(testing::read_request(&mut device));
        assert!(waiting.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    }
    testing::send_response(&mut device, sent[0].request_id, node::Response::Identified.into()).await;
    let released = testing::read_request(&mut device).await;
    testing::send_response(&mut device, released.request_id, node::Response::Identified.into()).await;
    for envelope in sent.iter().skip(1) {
        testing::send_response(&mut device, envelope.request_id, node::Response::Identified.into()).await;
    }
    for call in calls {
        call.await.unwrap().unwrap();
    }
}

#[tokio::test(start_paused = true)]
async fn dropping_handle_stream() {
    use tokio::io::AsyncWriteExt;

    let (connection, mut device) = testing::connected();
    drop(connection);

    // The reader task is aborted, not merely idle: with `tokio::io::split`
    // the shared duplex only drops once both halves do, so before the
    // fix the reader task (blocked on `next_frame`, unaffected by the
    // outbound channel closing) keeps the peer's writes succeeding
    // forever. After the fix, aborting it drops its read half too, so
    // the peer eventually observes the connection close.
    let observed_close = async {
        loop {
            if device.write(&[0]).await.is_err() {
                return;
            }
            tokio::task::yield_now().await;
        }
    };
    observed_close.await;
}

/* === Tests === */
