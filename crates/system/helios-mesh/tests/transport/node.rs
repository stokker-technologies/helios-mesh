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
use std::time::Duration;

// Third Party Imports

// Local Imports
use helios_mesh::testing;
use helios_mesh::{AuxGpioAccess, AuxGpioError, AuxGpioId, AuxGpioMode, Error, NodeSummary, NodeType, UsbDevice};
use helios_mesh_protocol::{
    error::Error as WireError,
    gateway,
    node::{self, ChannelId, NodeDescription, NodeId, OperationId, OperationSnapshot, OperationState},
    wire::Request,
};

/* === Constants === */

const NODE: NodeId = NodeId(566_114);
const OPERATION: OperationId = OperationId(7);
const CHANNEL: ChannelId = ChannelId(1);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn completed_snapshot() -> node::Response {
    node::Response::OperationStatus(OperationSnapshot {
        error: None,
        cleanup_error: None,
        id: OPERATION,
        channel: CHANNEL,
        state: OperationState::Completed,
    })
}

fn roster_page(silent_for_s: u16) -> gateway::NodePage {
    gateway::NodePage {
        nodes: heapless::Vec::from_slice(&[NodeSummary { info: testing::info(NODE), silent_for_s }]).unwrap(),
        next_after: None,
    }
}

fn device(id: u32, name: &str) -> UsbDevice {
    UsbDevice { id: NodeId(id), label: name.to_owned(), port: format!("/dev/tty.fake-{id}") }
}

fn description(coordinator: bool) -> NodeDescription {
    let mut info = testing::info(NodeId(0));
    info.node_type = if coordinator { NodeType::Gateway } else { NodeType::Sidecar };
    NodeDescription { info, satellites: heapless::Vec::new() }
}

#[tokio::test(start_paused = true)]
async fn gpio_pulse_admission() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::PulseAuxGpio(request)) if request.gpio == AuxGpioId(1) => node::Response::AuxGpioPulseScheduled {
            operation: helios_mesh_protocol::node::OperationId(1),
            gpio: request.gpio,
            duration_ms: request.duration_ms,
        }
        .into(),
        Request::Node(node::Request::PulseAuxGpio(request)) => WireError::AuxGpio { gpio: request.gpio, reason: AuxGpioError::Busy }.into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let gateway = testing::gateway(testing::node(connection, NodeId(273_282), true));
        let helios_mesh::ConnectedNode::Sidecar(sidecar) =
            gateway.connect_node(NodeSummary { info: testing::info(NODE), silent_for_s: 0 }).await.unwrap()
        else {
            panic!("the roster declares a Sidecar");
        };
        // A long pulse must acknowledge promptly, without a host sleep,
        // completion poll, or follow-up low command.
        sidecar.pulse_gpio(AuxGpioId(1), Duration::from_secs(60)).await.unwrap();
        sidecar.pulse_gpio(AuxGpioId(1), Duration::from_millis(1)).await.unwrap();
        assert!(matches!(
            sidecar.pulse_gpio(AuxGpioId(2), Duration::from_secs(1)).await,
            Err(Error::AuxGpio { gpio: AuxGpioId(2), reason: AuxGpioError::Busy })
        ));
    }
    let seen = responder.await.unwrap();
    assert_eq!(seen.len(), 3);
    assert!(seen.iter().all(|request| request.destination == NODE));
    for (request, expected_ms) in seen.iter().zip([60_000, 1, 1_000]) {
        assert!(matches!(
            request.request,
            Request::Node(node::Request::PulseAuxGpio(node::PulseAuxGpio {
                duration_ms, ..
            })) if duration_ms == expected_ms
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn invalid_pulse_transport() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| panic!("invalid pulse reached transport: {request:?}"));
    {
        let node = testing::node(connection, NODE, true);
        for duration in
            [Duration::ZERO, Duration::from_nanos(1), Duration::from_micros(1_500), Duration::from_millis(60_001), Duration::MAX]
        {
            assert!(matches!(
                testing::sidecar(node.clone()).pulse_gpio(AuxGpioId(1), duration).await,
                Err(Error::AuxGpio { reason: AuxGpioError::InvalidDuration, .. })
            ));
        }
        for gpio in [AuxGpioId(0), AuxGpioId(5)] {
            assert!(matches!(
                testing::sidecar(node.clone()).pulse_gpio(gpio, Duration::from_secs(1)).await,
                Err(Error::AuxGpio { reason: AuxGpioError::InvalidId, .. })
            ));
        }
    }
    assert_eq!(responder.await.unwrap(), [] as [helios_mesh_protocol::wire::RequestEnvelope; 0]);
}

#[tokio::test(start_paused = true)]
async fn mesh_gpio_permissions() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::ConfigureAuxGpio(request)) if request.gpio == AuxGpioId(1) => {
            node::Response::AuxGpioConfigured { gpio: request.gpio, mode: request.mode }.into()
        }
        Request::Node(node::Request::ConfigureAuxGpio(request)) => {
            WireError::AuxGpio { gpio: request.gpio, reason: AuxGpioError::Reserved }.into()
        }
        Request::Node(node::Request::Status(_)) => {
            let mut status = testing::status(NODE);
            status.auxiliary_gpio_access =
                [AuxGpioAccess::PushPull, AuxGpioAccess::Reserved, AuxGpioAccess::NotAllocated, AuxGpioAccess::InputOnly];
            node::Response::Status(status).into()
        }
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let gateway = testing::gateway(testing::node(connection, NodeId(273_282), true));
        let remote = gateway.connect_node(NodeSummary { info: testing::info(NODE), silent_for_s: 0 }).await.unwrap();
        assert_eq!(remote.node().status().await.unwrap().auxiliary_gpio_access[1], AuxGpioAccess::Reserved);
        testing::sidecar(remote.node().clone()).set_gpio(AuxGpioId(1), AuxGpioMode::DriveHigh).await.unwrap();
        assert!(matches!(
            testing::sidecar(remote.node().clone()).set_gpio(AuxGpioId(2), AuxGpioMode::DriveLow).await,
            Err(Error::AuxGpio { gpio: AuxGpioId(2), reason: AuxGpioError::Reserved })
        ));
        assert!(matches!(
            testing::sidecar(remote.node().clone()).set_gpio(AuxGpioId(0), AuxGpioMode::DriveLow).await,
            Err(Error::AuxGpio { gpio: AuxGpioId(0), reason: AuxGpioError::InvalidId })
        ));
    }
    let seen = responder.await.unwrap();
    assert_eq!(seen.len(), 3);
    assert!(seen.iter().all(|request| request.destination == NODE));
}

#[tokio::test(start_paused = true)]
async fn bound_elsewhere_identity() {
    const HOLDER: NodeId = NodeId(273_282);
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::ConfigureAuxGpio(_)) => WireError::BoundElsewhere { gateway: HOLDER }.into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, true);
        assert!(matches!(
            testing::sidecar(node.clone()).set_gpio(AuxGpioId(1), AuxGpioMode::DriveLow).await,
            Err(Error::BoundElsewhere { gateway: HOLDER })
        ));
    }
    responder.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn settled_reconciles_polling() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::OperationStatus(_)) => completed_snapshot().into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, true);
        let snapshot = node.operation(OPERATION).settled().await.unwrap();
        assert_eq!(snapshot.state, OperationState::Completed);
    }
    let seen = responder.await.unwrap();
    assert!(
        !seen.iter().any(|envelope| matches!(envelope.request, Request::Node(node::Request::Poll(_)))),
        "a remote operation must reconcile from the gateway's cache, not a mesh poll",
    );
}

#[tokio::test(start_paused = true)]
async fn settled_still_operation() {
    let (connection, device) = testing::connected();
    let responder = testing::respond(device, |request| match request {
        Request::Node(node::Request::Poll(_)) => node::Response::Polled.into(),
        Request::Node(node::Request::OperationStatus(_)) => completed_snapshot().into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let snapshot = node.operation(OPERATION).settled().await.unwrap();
        assert_eq!(snapshot.state, OperationState::Completed);
    }
    let seen = responder.await.unwrap();
    assert!(
        seen.iter().any(|envelope| matches!(envelope.request, Request::Node(node::Request::Poll(_)))),
        "a directly attached operation still polls to drain its local event queue",
    );
}

#[tokio::test(start_paused = true)]
async fn settled_fails_dark() {
    let (connection, device) = testing::connected();
    let start = tokio::time::Instant::now();
    let responder = testing::respond(device, move |request| match request {
        Request::Node(node::Request::OperationStatus(_)) => node::Response::OperationStatus(OperationSnapshot {
            error: None,
            cleanup_error: None,
            id: OPERATION,
            channel: CHANNEL,
            state: OperationState::Active,
        })
        .into(),
        Request::Gateway(gateway::Request::Nodes(_)) => {
            let elapsed = tokio::time::Instant::now().saturating_duration_since(start);
            gateway::Response::Nodes(roster_page(u16::try_from(elapsed.as_secs()).unwrap_or(u16::MAX))).into()
        }
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, true);
        let result = node.operation(OPERATION).settled().await;
        match result {
            Err(Error::NodeUnreachable { node: reported, silent_for_s }) => {
                assert_eq!(reported, NODE);
                assert!(silent_for_s > 12);
            }
            other => panic!("expected Error::NodeUnreachable, got {other:?}"),
        }
    }
    let seen = responder.await.unwrap();
    assert!(
        seen.iter()
            .any(|envelope| matches!(&envelope.request, Request::Gateway(gateway::Request::Nodes(_))) && envelope.destination == NODE),
        "the roster lookup rides the mesh-attached node's own handle and connection -- \
         confirmed answered locally by the coordinator regardless of destination (see \
         `crates/firmware/helios-mesh-core/src/node/mod.rs`'s envelope-level routing: any `Gateway` \
         request is `brokered` on every destination), not routed through a separate \
         gateway-addressed handle",
    );
}

#[tokio::test(start_paused = true)]
async fn settled_trip_fresh() {
    let (connection, device) = testing::connected();
    let start = tokio::time::Instant::now();
    let responder = testing::respond(device, move |request| match request {
        Request::Node(node::Request::OperationStatus(_)) => {
            let elapsed = tokio::time::Instant::now().saturating_duration_since(start);
            let state = if elapsed >= Duration::from_secs(20) { OperationState::Completed } else { OperationState::Active };
            node::Response::OperationStatus(OperationSnapshot { error: None, cleanup_error: None, id: OPERATION, channel: CHANNEL, state })
                .into()
        }
        Request::Gateway(gateway::Request::Nodes(_)) => gateway::Response::Nodes(roster_page(1)).into(),
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, true);
        let snapshot = node.operation(OPERATION).settled().await.unwrap();
        assert_eq!(snapshot.state, OperationState::Completed);
    }
    let seen = responder.await.unwrap();
    let roster_checks = seen.iter().filter(|envelope| matches!(envelope.request, Request::Gateway(gateway::Request::Nodes(_)))).count();
    // A 20s wait against a 5s `ROSTER_SILENCE_CHECK_INTERVAL` checks at
    // ~5s, ~10s, and ~15s -- the operation completes on the very poll
    // that first observes 20s elapsed, before a fourth check at ~20s
    // would fire. Exact, not a loose lower bound: pins the cadence
    // itself, not just "it happened more than once".
    assert_eq!(
        roster_checks, 3,
        "a 20s wait at a 5s cadence must check the roster exactly 3 times \
         (~5s, ~10s, ~15s), got {roster_checks}"
    );
}

#[tokio::test(start_paused = true)]
async fn settled_never_node() {
    let (connection, device) = testing::connected();
    let start = tokio::time::Instant::now();
    let responder = testing::respond(device, move |request| match request {
        Request::Node(node::Request::Poll(_)) => node::Response::Polled.into(),
        Request::Node(node::Request::OperationStatus(_)) => {
            let elapsed = tokio::time::Instant::now().saturating_duration_since(start);
            let state = if elapsed >= Duration::from_secs(20) { OperationState::Completed } else { OperationState::Active };
            node::Response::OperationStatus(OperationSnapshot { error: None, cleanup_error: None, id: OPERATION, channel: CHANNEL, state })
                .into()
        }
        other => panic!("unexpected request: {other:?}"),
    });
    {
        let node = testing::node(connection, NODE, false);
        let snapshot = node.operation(OPERATION).settled().await.unwrap();
        assert_eq!(snapshot.state, OperationState::Completed);
    }
    responder.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn select_probed_reopening() {
    let winner_id = NodeId(273_282);
    let loser_id = NodeId(566_114);
    let devices = [device(winner_id.0, "Gateway"), device(loser_id.0, "Rotator")];

    let (winner_connection, winner_device) = testing::connected();
    let (loser_connection, _loser_device) = testing::connected();
    let responder = testing::respond(winner_device, |request| match request {
        Request::Node(node::Request::Identify(_)) => node::Response::Identified.into(),
        other => panic!("unexpected request: {other:?}"),
    });
    let probes = vec![
        Some((testing::node(winner_connection, winner_id, false), Some(description(true)))),
        Some((testing::node(loser_connection, loser_id, false), Some(description(false)))),
    ];

    {
        let resolved = testing::select_probed(&devices, probes).await.unwrap();
        assert_eq!(resolved.id(), winner_id);
        // `select_probed` has no way to construct a fresh `Connection`;
        // this round trip only succeeds if the returned handle still
        // carries the exact connection its winning probe opened.
        resolved.identify().await.unwrap();
    }
    let seen = responder.await.unwrap();
    assert_eq!(seen.len(), 1, "the reused connection serves the post-selection request directly");
}

#[tokio::test(start_paused = true)]
async fn select_probed_returning() {
    use tokio::io::AsyncWriteExt;

    let winner_id = NodeId(273_282);
    let loser_id = NodeId(566_114);
    let devices = [device(winner_id.0, "Gateway"), device(loser_id.0, "Rotator")];

    let (winner_connection, _winner_device) = testing::connected();
    let (loser_connection, mut loser_device) = testing::connected();
    let probes = vec![
        Some((testing::node(winner_connection, winner_id, false), Some(description(true)))),
        Some((testing::node(loser_connection, loser_id, false), Some(description(false)))),
    ];

    let resolved = testing::select_probed(&devices, probes).await.unwrap();
    assert_eq!(resolved.id(), winner_id);

    assert!(
        loser_device.write(&[0]).await.is_err(),
        "a write from the fake device's side must fail once the losing candidate's \
         reader task has actually dropped its stream half"
    );
}

#[tokio::test(start_paused = true)]
async fn select_probed_fails() {
    use tokio::io::AsyncWriteExt;

    let first_id = NodeId(273_282);
    let second_id = NodeId(566_114);
    let devices = [device(first_id.0, "Rotator"), device(second_id.0, "Sidecar")];

    let (first_connection, mut first_device) = testing::connected();
    let (second_connection, mut second_device) = testing::connected();
    let probes = vec![
        Some((testing::node(first_connection, first_id, false), Some(description(false)))),
        Some((testing::node(second_connection, second_id, false), Some(description(false)))),
    ];

    assert!(matches!(testing::select_probed(&devices, probes).await, Err(Error::DeviceNotFound)));

    for device in [&mut first_device, &mut second_device] {
        assert!(device.write(&[0]).await.is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn select_probed_opened() {
    use tokio::io::AsyncWriteExt;

    let winner_id = NodeId(273_282);
    let unprobed_id = NodeId(566_114);
    let devices = [device(winner_id.0, "Gateway"), device(unprobed_id.0, "Unresponsive")];

    let (winner_connection, _winner_device) = testing::connected();
    let (unprobed_connection, mut unprobed_device) = testing::connected();
    let probes = vec![
        Some((testing::node(winner_connection, winner_id, false), Some(description(true)))),
        // `None` here is exactly what `NodeHandle::probe` returns for a
        // candidate whose `describe()` failed after a successful open.
        Some((testing::node(unprobed_connection, unprobed_id, false), None)),
    ];

    let resolved = testing::select_probed(&devices, probes).await.unwrap();
    assert_eq!(resolved.id(), winner_id);

    assert!(unprobed_device.write(&[0]).await.is_err(), "the unprobed candidate's connection was left open instead of closed");
}

/* === Tests === */
