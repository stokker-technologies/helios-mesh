/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Own Gateway Scheduling, Discovery, Correlation And Retry State.

/* === Modules === */

mod core;

/* === Re-Exports === */

pub use core::{
    DISCOVERY_COLLECTION_WINDOW_MS, DiscoveryOutcome, Error as CoordinatorError, FailureOutcome, GatewayCore, MAX_MANAGED_NODES,
    MAX_QUEUED_REQUESTS, MAX_TRACKED_OPERATIONS, NodeRecord, OperationCache, Priority, ScheduledRequest,
};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer};
use lora_phy::{DelayNs, mod_traits::RadioKind};

// Local Imports
use super::{
    link::{Error as LinkError, Link},
    retry::{BACKGROUND, INTERACTIVE, Policy},
};
use crate::{
    diagnostics::Diagnostics,
    node::{Handle as NodeHandle, Node, node_info, node_summary, services::peers},
    ui::RoundTripWindow,
};
use hnm_protocol::{
    diagnostics::JournalEvent,
    error::Error,
    gateway, motion,
    node::{self as wire_node, ChannelId, EventId, NodeDescription, NodeId, NodeInfo, OperationId, OperationState, RequestId, SessionId},
    wire::{
        Event, EventPayload, PROTOCOL_VERSION, Request, RequestEnvelope, Response, ResponseEnvelope,
        mesh::{Envelope, Message},
    },
};

/* === Constants === */

const MAINTENANCE_INTERVAL_MS: u64 = 30_000;
const OPERATION_EVENT_POLL_INTERVAL_MS: u64 = 250;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForwardQueueFull;

#[derive(Clone, Debug)]
struct Exchange {
    response: Response,
}

#[derive(Clone, Copy)]
struct ExchangeRoute {
    source: NodeId,
    source_session: SessionId,
    destination: NodeId,
    request_id: RequestId,
}

#[derive(Clone, Copy)]
struct PeerCursor {
    session: SessionId,
    event: EventId,
}

struct Shared {
    requests: Channel<CriticalSectionRawMutex, RequestEnvelope, MAX_QUEUED_REQUESTS>,
}

struct Inner<RK, DLY>
where
    RK: RadioKind,
    DLY: DelayNs,
{
    link: Link<RK, DLY>,
    core: GatewayCore,
    round_trip: RoundTripWindow,
    interactive: Policy,
    background: Policy,
    discovery_request_id: RequestId,
}

pub struct Coordinator<RK, DLY>
where
    RK: RadioKind,
    DLY: DelayNs,
{
    shared: Shared,
    inner: Inner<RK, DLY>,
}

#[derive(Clone, Copy)]
pub struct Handle {
    shared: &'static Shared,
}

pub struct Runner<RK, DLY>
where
    RK: RadioKind + 'static,
    DLY: DelayNs + 'static,
{
    shared: &'static Shared,
    inner: &'static mut Inner<RK, DLY>,
}

struct BroadcastReply {
    source: NodeId,
    source_session: SessionId,
    description: NodeDescription,
    event: Option<Event>,
    request_id: RequestId,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PeerCursor {
    const fn new() -> Self {
        Self { session: SessionId::UNKNOWN, event: EventId(0) }
    }
}

impl<RK, DLY> Coordinator<RK, DLY>
where
    RK: RadioKind,
    DLY: DelayNs,
{
    #[must_use]
    pub const fn new(link: Link<RK, DLY>) -> Self {
        Self {
            shared: Shared { requests: Channel::new() },
            inner: Inner {
                link,
                core: GatewayCore::new(MAINTENANCE_INTERVAL_MS),
                round_trip: RoundTripWindow::new(),
                interactive: INTERACTIVE,
                background: BACKGROUND,
                discovery_request_id: RequestId(1),
            },
        }
    }

    pub fn split(&'static mut self) -> (Runner<RK, DLY>, Handle) {
        let shared: &'static Shared = &self.shared;
        let inner: &'static mut Inner<RK, DLY> = &mut self.inner;
        (Runner { shared, inner }, Handle { shared })
    }
}

impl Handle {
    pub fn try_forward(&self, request: RequestEnvelope) -> Result<(), ForwardQueueFull> {
        self.shared.requests.try_send(request).map_err(|_| ForwardQueueFull)
    }
}

impl<RK, DLY> Runner<RK, DLY>
where
    RK: RadioKind + 'static,
    DLY: DelayNs + 'static,
{
    pub async fn run<R: Node + 'static>(mut self, node: NodeHandle<R>) -> ! {
        let gateway = node_info::<R>(node.node_id());
        let gateway_session = node.session_id();
        loop {
            while let Ok(request) = self.shared.requests.try_receive() {
                self.accept_host_request(&gateway, node, request).await;
            }

            let now_ms = Instant::now().as_millis();
            if let Some(scheduled) = self.inner.core.take_next(now_ms) {
                self.service_scheduled(&gateway, gateway_session, node, scheduled).await;
                continue;
            }

            if let Some(due) = self.inner.core.due_maintenance(now_ms) {
                let priority = if self.inner.core.operations().has_active(due) { Priority::Operation } else { Priority::Maintenance };
                let _ = self.inner.core.enqueue(ScheduledRequest::new(
                    due,
                    maintenance_request(&self.inner.core, due, now_ms),
                    priority,
                    now_ms,
                    now_ms,
                ));
                continue;
            }

            // Reached only with an empty ready queue and no maintenance due,
            // so a broadcast sweep never displaces host or maintenance work;
            // the cadence itself is the core's registry-driven policy.
            if self.inner.core.due_discovery(now_ms) {
                let _ = self.inner.core.enqueue(ScheduledRequest::new(
                    NodeId::BROADCAST,
                    Request::Node(wire_node::Request::Describe(wire_node::Describe)),
                    Priority::Discovery,
                    now_ms,
                    now_ms,
                ));
                self.inner.core.defer_discovery(now_ms);
                continue;
            }

            let mut wake_ms = self.inner.core.next_discovery_deadline();
            if let Some(deadline) = self.inner.core.next_maintenance_deadline() {
                wake_ms = wake_ms.min(deadline);
            }
            if let Some(deadline) = self.inner.core.next_request_deadline() {
                wake_ms = wake_ms.min(deadline);
            }
            match select(self.shared.requests.receive(), Timer::at(Instant::from_millis(wake_ms))).await {
                Either::First(request) => {
                    self.accept_host_request(&gateway, node, request).await;
                }
                Either::Second(()) => {}
            }
        }
    }

    async fn accept_host_request<R: Node + 'static>(&mut self, gateway: &NodeInfo, node: NodeHandle<R>, envelope: RequestEnvelope) {
        let reply = envelope.request_id;
        // Node paging is gateway semantics beyond forwarding: answer it from
        // the authoritative registry without a radio turn. Roster entries
        // carry capabilities straight from each record's cached
        // `NodeDescription` (protocol v7), so this stays a zero-air read.
        if let Request::Gateway(gateway::Request::Nodes(query)) = &envelope.request {
            let summary = node_summary::<R>(gateway.id);
            let now_ms = Instant::now().as_millis();
            let page = self.inner.core.nodes(query.after, &summary, now_ms);
            respond(node, reply, gateway::Response::Nodes(page).into()).await;
            return;
        }
        // Remote operation status answered from the local projection when the
        // operation is known; otherwise ask the node itself.
        if let Request::Node(wire_node::Request::OperationStatus(query)) = &envelope.request
            && let Some(snapshot) = self.inner.core.operations().status(envelope.destination, query.operation)
            && snapshot.state.is_terminal()
        {
            respond(node, reply, wire_node::Response::OperationStatus(snapshot).into()).await;
            return;
        }

        let destination = envelope.destination;
        let priority = if self.inner.core.contains(destination) {
            if matches!(envelope.request, Request::Motion(motion::Request::Stop(_))) { Priority::Emergency } else { Priority::Interactive }
        } else if matches!(envelope.request, Request::Node(wire_node::Request::Describe(_))) {
            Priority::Discovery
        } else {
            respond(node, reply, Error::NotFound.into()).await;
            return;
        };
        let now_ms = Instant::now().as_millis();
        let scheduled =
            ScheduledRequest::new(destination, envelope.request, priority, now_ms, now_ms.saturating_add(100)).with_reply(reply);
        // `enqueue_checked` also refuses a motion admission the operation
        // cache could not retain in its worst case, *before* the node ever
        // sees the request: forwarding it regardless and only catching a
        // tracking failure after the node has already accepted and started
        // real motion would leave that motion committed but unreachable by
        // id (worst case for `MoveTogether`, which the node accepts as one
        // atomic pair). `observe_operation_response`'s post-hoc override
        // remains as defense in depth for the gap between this check and
        // the exchange actually completing, when capacity can still move.
        if self.inner.core.enqueue_checked(scheduled).is_err() {
            respond(node, reply, Error::Busy.into()).await;
        }
    }

    async fn service_scheduled<R: Node + 'static>(
        &mut self, gateway: &NodeInfo, gateway_session: SessionId, node: NodeHandle<R>, scheduled: ScheduledRequest,
    ) {
        let diagnostics = node.diagnostics();
        if !self.inner.core.contains(scheduled.node) {
            self.service_discovery(gateway, gateway_session, node, &scheduled).await;
            return;
        }

        let allocated = self.inner.core.allocate_request_id(scheduled.node).ok();
        let acknowledged = self.inner.core.acknowledgement(scheduled.node).ok();
        let (Some(request_id), Some((session, event))) = (allocated, acknowledged) else {
            // The registry check above makes this unreachable; still answer,
            // because a dropped reply strands the chassis forward entry.
            if let Some(reply) = scheduled.reply {
                respond(node, reply, unreachable_error(true).into()).await;
            }
            return;
        };
        let mut cursor = PeerCursor { session, event };
        let route = ExchangeRoute { source: gateway.id, source_session: gateway_session, destination: scheduled.node, request_id };
        let policy = if matches!(scheduled.priority, Priority::Maintenance | Priority::Discovery) {
            self.inner.background
        } else {
            self.inner.interactive
        };
        let exchange = self.exchange(route, scheduled.request.clone(), &mut cursor, policy, diagnostics).await;
        let now_ms = Instant::now().as_millis();
        let _ = self.inner.core.mark_served(scheduled.node);
        let Some(exchange) = exchange else {
            if self.inner.core.observe_failure(scheduled.node, now_ms) == Ok(FailureOutcome::Evicted) {
                diagnostics.record(JournalEvent::NodeEvicted { node: scheduled.node });
            }
            self.schedule_operation_collection(scheduled.node, now_ms);
            if let Some(reply) = scheduled.reply {
                respond(node, reply, unreachable_error(true).into()).await;
            }
            return;
        };
        let _ = self.inner.core.observe_cursor(scheduled.node, cursor.session, cursor.event, now_ms);
        let override_response =
            observe_operation_response(&mut self.inner.core, scheduled.node, &scheduled.request, &exchange.response, now_ms, diagnostics);
        self.schedule_operation_collection(scheduled.node, now_ms);
        self.push_nearby(node, scheduled.node).await;
        if let Some(stats) = self.inner.round_trip.stats() {
            node.observe_round_trip(stats).await;
        }
        if let Some(reply) = scheduled.reply {
            respond(node, reply, override_response.unwrap_or(exchange.response)).await;
        }
    }

    async fn service_discovery<R: Node + 'static>(
        &mut self, gateway: &NodeInfo, gateway_session: SessionId, node: NodeHandle<R>, scheduled: &ScheduledRequest,
    ) {
        if scheduled.node == NodeId::BROADCAST {
            self.service_broadcast_discovery(gateway, gateway_session, node).await;
            return;
        }
        let diagnostics = node.diagnostics();
        let reply = scheduled.reply;
        let request_id = self.inner.discovery_request_id;
        let policy = if reply.is_some() { self.inner.interactive } else { self.inner.background };
        let discovered = self.discover(gateway.id, gateway_session, scheduled.node, request_id, policy, diagnostics).await;
        advance(&mut self.inner.discovery_request_id);
        let response = if let Some((description, cursor)) = discovered {
            let discovered_node = description.info.id;
            let now_ms = Instant::now().as_millis();
            if self.inner.core.absorb_discovery(description.clone(), cursor.session, cursor.event, request_id, now_ms).is_ok() {
                self.push_nearby(node, discovered_node).await;
                wire_node::Response::Description(description).into()
            } else {
                Error::Busy.into()
            }
        } else {
            unreachable_error(false).into()
        };
        if let Some(reply) = reply {
            respond(node, reply, response).await;
        }
    }

    async fn discover(
        &mut self, gateway_id: NodeId, gateway_session: SessionId, destination: NodeId, request_id: RequestId, policy: Policy,
        diagnostics: &Diagnostics,
    ) -> Option<(NodeDescription, PeerCursor)> {
        let mut cursor = PeerCursor::new();
        let request = Request::Node(wire_node::Request::Describe(wire_node::Describe));
        let route = ExchangeRoute { source: gateway_id, source_session: gateway_session, destination, request_id };
        let (source, exchange) = self.exchange_from(route, request, &mut cursor, policy, diagnostics).await?;
        discovered_description(source, exchange.response).map(|description| (description, cursor))
    }

    async fn service_broadcast_discovery<R: Node + 'static>(
        &mut self, gateway: &NodeInfo, gateway_session: SessionId, node: NodeHandle<R>,
    ) {
        let diagnostics = node.diagnostics();
        let request_id = self.inner.discovery_request_id;
        advance(&mut self.inner.discovery_request_id);
        let packet = Envelope {
            version: PROTOCOL_VERSION,
            source: gateway.id,
            session: gateway_session,
            destination: NodeId::BROADCAST,
            request_id,
            message: Message::Request {
                request: Request::Node(wire_node::Request::Describe(wire_node::Describe)),
                acknowledged_session: SessionId::UNKNOWN,
                acknowledged_event: EventId(0),
            },
        };
        if self.inner.link.send(&packet).await.is_err() {
            return;
        }
        let timing = self.inner.link.last_send_timing();
        diagnostics.record(JournalEvent::MeshTransmit {
            peer: NodeId::BROADCAST,
            request_id,
            turnaround_ms: timing.turnaround_ms,
            preparation_ms: timing.preparation_ms,
            transmit_ms: timing.transmit_ms,
        });

        let deadline = Instant::now() + Duration::from_millis(DISCOVERY_COLLECTION_WINDOW_MS);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining == Duration::MIN {
                return;
            }
            let Ok(incoming) = self.inner.link.receive_timeout(remaining).await else {
                return;
            };
            if incoming.destination != gateway.id
                || !incoming.is_supported()
                || !incoming.source.is_device()
                || incoming.request_id != request_id
            {
                continue;
            }
            let Message::Response { response, event } = incoming.message else {
                continue;
            };
            let Some(description) = discovered_description(incoming.source, response) else {
                continue;
            };
            let now_ms = Instant::now().as_millis();
            if absorb_broadcast_response(
                &mut self.inner.core,
                BroadcastReply { source: incoming.source, source_session: incoming.session, description, event, request_id },
                diagnostics,
                now_ms,
            )
            .is_ok()
            {
                self.push_nearby(node, incoming.source).await;
            }
        }
    }

    async fn exchange(
        &mut self, route: ExchangeRoute, request: Request, cursor: &mut PeerCursor, policy: Policy, diagnostics: &Diagnostics,
    ) -> Option<Exchange> {
        self.exchange_from(route, request, cursor, policy, diagnostics).await.map(|(_, exchange)| exchange)
    }

    async fn exchange_from(
        &mut self, route: ExchangeRoute, request: Request, cursor: &mut PeerCursor, policy: Policy, diagnostics: &Diagnostics,
    ) -> Option<(NodeId, Exchange)> {
        let inner = &mut *self.inner;
        let started = Instant::now();
        let packet = Envelope {
            version: PROTOCOL_VERSION,
            source: route.source,
            session: route.source_session,
            destination: route.destination,
            request_id: route.request_id,
            message: Message::Request { request, acknowledged_session: cursor.session, acknowledged_event: cursor.event },
        };

        for attempt in 0..policy.attempts {
            if attempt > 0 {
                diagnostics.record(JournalEvent::RequestRetried { peer: route.destination, request_id: route.request_id, retry: attempt });
                Timer::after(policy.delay()).await;
            }
            if inner.link.send(&packet).await.is_err() {
                continue;
            }
            let timing = inner.link.last_send_timing();
            diagnostics.record(JournalEvent::MeshTransmit {
                peer: route.destination,
                request_id: route.request_id,
                turnaround_ms: timing.turnaround_ms,
                preparation_ms: timing.preparation_ms,
                transmit_ms: timing.transmit_ms,
            });
            loop {
                let incoming = match inner.link.receive_timeout(policy.response_timeout).await {
                    Ok(incoming) => incoming,
                    Err(LinkError::Timeout) => break,
                    Err(_) => return None,
                };
                if incoming.destination != route.source
                    || !incoming.is_supported()
                    || !incoming.source.is_device()
                    || (route.destination != NodeId::BROADCAST && incoming.source != route.destination)
                    || incoming.request_id != route.request_id
                {
                    continue;
                }
                if let Message::Response { response, event } = incoming.message {
                    let duration_ms = started.elapsed().as_millis();
                    diagnostics.record(JournalEvent::MeshRoundTrip {
                        peer: incoming.source,
                        request_id: route.request_id,
                        duration_ms: u32::try_from(duration_ms).unwrap_or(u32::MAX),
                    });
                    inner.round_trip.record(u32::try_from(duration_ms).unwrap_or(u32::MAX));
                    observe_event(&mut inner.core, cursor, incoming.source, incoming.session, incoming.request_id, event, diagnostics);
                    return Some((incoming.source, Exchange { response }));
                }
            }
        }
        None
    }

    fn schedule_operation_collection(&mut self, node: NodeId, now_ms: u64) {
        if self.inner.core.operations().has_active(node) {
            let _ = self.inner.core.bring_maintenance_forward(node, now_ms.saturating_add(OPERATION_EVENT_POLL_INTERVAL_MS));
        }
    }

    async fn push_nearby<R: Node + 'static>(&self, node: NodeHandle<R>, id: NodeId) {
        let Some(record) = self.inner.core.node(id) else {
            return;
        };
        node.observe_nearby(peers::Record {
            id,
            label: record.description.info.label().try_into().expect("catalog label fits"),
            last_seen_ms: record.last_seen_ms,
        })
        .await;
    }
}

/* === Public Functions === */

/* === Private Functions === */

async fn respond<R: Node + 'static>(node: NodeHandle<R>, request_id: RequestId, response: Response) {
    node.remote_response(ResponseEnvelope { version: PROTOCOL_VERSION, request_id, response }).await;
}

const fn unreachable_error(registered: bool) -> Error {
    if registered { Error::Unavailable } else { Error::NotFound }
}

fn discovered_description(source: NodeId, response: Response) -> Option<NodeDescription> {
    match response {
        Response::Node(wire_node::Response::Description(description)) if description.info.id == source && source.is_device() => {
            Some(description)
        }
        _ => None,
    }
}

fn observe_event(
    core: &mut GatewayCore, cursor: &mut PeerCursor, source: NodeId, source_session: SessionId, request_id: RequestId,
    event: Option<Event>, diagnostics: &Diagnostics,
) {
    if source_session != cursor.session {
        core.operations_mut().clear_node(source);
        cursor.session = source_session;
        cursor.event = EventId(0);
    }
    let Some(event) = event else {
        return;
    };
    if event.id.0 <= cursor.event.0 {
        return;
    }
    cursor.event = event.id;
    observe_operation_event(core, source, event, Instant::now().as_millis());
    diagnostics.record(JournalEvent::CapabilityEventReturned { peer: source, request_id, event });
}

fn absorb_broadcast_response(
    core: &mut GatewayCore, reply: BroadcastReply, diagnostics: &Diagnostics, now_ms: u64,
) -> Result<DiscoveryOutcome, self::core::Error> {
    let BroadcastReply { source, source_session, description, event, request_id } = reply;
    if core.contains(source) {
        return core.absorb_discovery(description, source_session, EventId(0), request_id, now_ms);
    }
    let mut cursor = PeerCursor::new();
    observe_event(core, &mut cursor, source, source_session, request_id, event, diagnostics);
    core.absorb_discovery(description, cursor.session, cursor.event, request_id, now_ms)
}

fn maintenance_request(core: &GatewayCore, node: NodeId, now_ms: u64) -> Request {
    core.operations().stale_active(node, now_ms).map_or(Request::Node(wire_node::Request::Status(wire_node::Status)), |operation| {
        Request::Node(wire_node::Request::OperationStatus(wire_node::OperationStatus { operation }))
    })
}

fn observe_operation_response(
    core: &mut GatewayCore, node: NodeId, request: &Request, response: &Response, now_ms: u64, diagnostics: &Diagnostics,
) -> Option<Response> {
    let operations = core.operations_mut();
    let track = |operations: &mut OperationCache, operation: OperationId, channel: ChannelId| {
        let tracked = operations.accepted(node, operation, channel, now_ms).is_ok();
        if !tracked {
            diagnostics.record(JournalEvent::OperationCacheSaturated { node, operation });
        }
        tracked
    };
    match (request, response) {
        (
            Request::Node(wire_node::Request::PulseAuxGpio(request)),
            Response::Node(wire_node::Response::AuxGpioPulseScheduled { operation, .. }),
        ) => (!track(operations, *operation, ChannelId(request.gpio.0))).then(|| Error::Busy.into()),
        // A `Stop`'s own operation is tracked exactly like any other
        // admission -- no bulk side-inference about the channel's
        // previously active operation. The node is the sole authority on
        // whether (and which) operation actually stopped: its `Stopped`
        // event names the operation it interrupted, if any
        // (`motor::Outcome::Stopped`'s `interrupted` field, reflected in
        // the node's own ring), and any operation this cache is still
        // tracking as Active on the same channel self-corrects through the
        // existing stale-operation reconciliation (`stale_active`,
        // `RECONCILE_AFTER_MS`) instead of a same-instant guess that can be
        // wrong for an operation the stop never touched.
        (
            Request::Motion(
                motion::Request::MoveTo(motion::MoveTo { channel, .. })
                | motion::Request::MoveRelative(motion::MoveRelative { channel, .. })
                | motion::Request::SetHome(motion::SetHome { channel, .. })
                | motion::Request::Stop(motion::Stop { channel, .. }),
            ),
            Response::Motion(motion::Response::Accepted { operation }),
        ) => (!track(operations, *operation, *channel)).then(|| Error::Busy.into()),
        (
            Request::Motion(motion::Request::MoveTogether(request)),
            Response::Motion(motion::Response::AcceptedTogether { operations: accepted }),
        ) => {
            let mut saturated = false;
            for (movement, operation) in request.moves.iter().zip(accepted.iter()) {
                if !track(operations, *operation, movement.channel) {
                    saturated = true;
                }
            }
            saturated.then(|| Error::Busy.into())
        }
        (Request::Node(wire_node::Request::OperationStatus(_)), Response::Node(wire_node::Response::OperationStatus(status))) => {
            // Already-real information from the node itself (its true
            // current state), forwarded unchanged either way: whether the
            // cache can retain it only affects whether the *next* poll is a
            // free cache hit or another live exchange, never the honesty of
            // this answer.
            operations.observe(node, *status, now_ms);
            None
        }
        // A reconciliation query (see `maintenance_request`) found the node
        // no longer remembers the operation at all: its own bounded history
        // has rotated past it, almost always because it finished long
        // enough ago that this gateway's projection fell behind truth. This
        // is not "never existed" — the gateway itself accepted it — so it
        // must never surface as `NotFound` to a host still waiting on it
        // (reserved for operations that were never accepted, or a node that
        // left the registry). `Failed` is the honest mapping: the true
        // outcome is now unknowable, so the host gets an actionable
        // `OperationFailed` rather than either hanging on a stale `Active`
        // forever or receiving a misleading raw `NotFound`. A genuine
        // session change (the node actually rebooted) already cleared this
        // node's whole projection earlier in the same exchange
        // (`observe_event`'s `clear_node`), so `operations.status` finding
        // nothing here means exactly "not tracked" and this is a no-op —
        // including the case this same fix closes off going forward: an
        // admission the cache never retained in the first place no longer
        // reaches a host as a real-looking `Accepted`, so it can no longer
        // leave a host still waiting on an id only this arm could recover.
        (Request::Node(wire_node::Request::OperationStatus(query)), Response::Error(Error::NotFound)) => {
            if let Some(snapshot) = operations.status(node, query.operation) {
                operations.terminal(node, query.operation, snapshot.channel, OperationState::Failed, now_ms);
            }
            None
        }
        _ => None,
    }
}

fn observe_operation_event(core: &mut GatewayCore, node: NodeId, event: Event, now_ms: u64) {
    let (channel, operation, state) = match event.payload {
        EventPayload::Motion(motion::Event::Homed { channel, operation } | motion::Event::PositionReached { channel, operation, .. }) => {
            (channel, operation, OperationState::Completed)
        }
        EventPayload::Motion(motion::Event::Stopped { channel, operation }) => (channel, operation, OperationState::Stopped),
        EventPayload::Motion(motion::Event::Failed { channel, operation }) => (channel, operation, OperationState::Failed),
    };
    core.operations_mut().terminal(node, operation, channel, state, now_ms);
}

fn advance(request_id: &mut RequestId) {
    request_id.0 = request_id.0.wrapping_add(1).max(1);
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::node::{ChannelId, FIRMWARE_VERSION, OperationId, OperationSnapshot};

    use super::*;

    #[test]
    fn failed_exchange_retryable() {
        assert_eq!(unreachable_error(true), Error::Unavailable);
        assert_eq!(unreachable_error(false), Error::NotFound);
    }

    fn describe(id: u32) -> NodeDescription {
        NodeDescription {
            info: NodeInfo { node_type: hnm_protocol::nodes::NodeType::Sidecar, id: NodeId(id), firmware: FIRMWARE_VERSION },

            satellites: heapless::Vec::new(),
        }
    }

    #[test]
    fn broadcast_reply_operations() {
        let node = NodeId(566_114);
        let diagnostics = Diagnostics::new();
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        core.observe_cursor(node, SessionId(42), EventId(7), 0).unwrap();
        core.operations_mut().accepted(node, OperationId(1), ChannelId(1), 0).unwrap();

        // Exactly what one broadcast sweep's collection window does for
        // every response it hears: `service_broadcast_discovery` calling
        // `absorb_broadcast_response` with this node's real, unchanged
        // session and no piggybacked capability event.
        let outcome = absorb_broadcast_response(
            &mut core,
            BroadcastReply {
                source: node,
                source_session: SessionId(42),
                description: describe(node.0),
                event: None,
                request_id: RequestId(9),
            },
            &diagnostics,
            2_000,
        )
        .unwrap();

        assert_eq!(outcome, DiscoveryOutcome::LivenessRefreshed);
        assert_eq!(
            core.acknowledgement(node).unwrap(),
            (SessionId(42), EventId(7)),
            "the event cursor must be untouched by a broadcast echo"
        );
        assert!(
            core.operations().status(node, OperationId(1)).is_some(),
            "a broadcast echo from an already-registered node must not clear its tracked operations"
        );
        assert!(core.operations().has_active(node));
    }

    #[test]
    fn discovery_accepts_devices() {
        let source = NodeId(123_456);
        let description = NodeDescription {
            info: NodeInfo { node_type: hnm_protocol::nodes::NodeType::Sidecar, id: source, firmware: FIRMWARE_VERSION },

            satellites: heapless::Vec::new(),
        };

        assert_eq!(
            discovered_description(source, Response::Node(wire_node::Response::Description(description.clone())),),
            Some(description.clone())
        );
        assert_eq!(discovered_description(NodeId(654_321), Response::Node(wire_node::Response::Description(description)),), None);
    }

    #[test]
    fn stalled_operation_cadence() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let channel = ChannelId(1);
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        core.operations_mut().accepted(node, operation, channel, 0).unwrap();

        let mut now_ms = 0;
        let mut reconciled = false;
        for _ in 0..40 {
            // The real drain cadence: one operation-collection exchange
            // every OPERATION_EVENT_POLL_INTERVAL_MS. The completion event
            // itself never arrives at any of these ticks -- it was lost at
            // the node, permanently, before this simulation even starts.
            now_ms += OPERATION_EVENT_POLL_INTERVAL_MS;
            match maintenance_request(&core, node, now_ms) {
                Request::Node(wire_node::Request::OperationStatus(query)) => {
                    // The on-air exchange succeeds and the node reports its
                    // true, already-completed state -- exactly what
                    // `observe_operation_response`'s `OperationStatus`
                    // branch applies for a real reconciliation response.
                    core.operations_mut().accepted(node, query.operation, channel, now_ms).unwrap();
                    core.operations_mut().terminal(node, query.operation, channel, OperationState::Completed, now_ms);
                    reconciled = true;
                    break;
                }
                Request::Node(wire_node::Request::Status(_)) => {
                    // A plain dynamic-status poll carries no operation
                    // information at all: it cannot, by itself, ever
                    // resolve this operation without the lost event.
                }
                other => panic!("unexpected maintenance request: {other:?}"),
            }
        }

        assert!(reconciled, "the operation must reconcile within a bounded number of collection ticks");
        assert_eq!(core.operations().status(node, operation).unwrap().state, OperationState::Completed);
        assert!(!core.operations().has_active(node));
    }

    #[test]
    fn reconciliation_notfound_notfound() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let channel = ChannelId(1);
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        core.operations_mut().accepted(node, operation, channel, 0).unwrap();
        let diagnostics = Diagnostics::new();

        let override_response = observe_operation_response(
            &mut core,
            node,
            &Request::Node(wire_node::Request::OperationStatus(wire_node::OperationStatus { operation })),
            &Response::Error(Error::NotFound),
            2_000,
            &diagnostics,
        );
        assert_eq!(override_response, None, "the reconciliation remap only updates the cache; nothing new is sent to a host here");

        // The host-visible outcome must never be a raw NotFound for an
        // operation the gateway itself accepted: the cache still holds it,
        // so the host's next cache read is answered from here, not
        // forwarded to the node (where it would hit the same NotFound).
        let snapshot = core.operations().status(node, operation).expect("a gateway-accepted operation must stay resolvable from the cache");
        assert_eq!(snapshot.state, OperationState::Failed, "an unknowable outcome must resolve to an actionable terminal state");
        assert!(!core.operations().has_active(node));
    }

    #[test]
    fn accepting_stop_channel() {
        let node = NodeId(566_114);
        let channel = ChannelId(2);
        let unrelated_operation = OperationId(294);
        let stop_operation = OperationId(300);
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        let diagnostics = Diagnostics::new();

        // The retried admission's acceptance, tracked normally: this
        // operation has nothing to do with the stop that follows.
        core.operations_mut().accepted(node, unrelated_operation, channel, 207_000).unwrap();

        // A `Stop` on the same channel is accepted -- for its own,
        // different operation. Nothing about this exchange names
        // `unrelated_operation`.
        let response = observe_operation_response(
            &mut core,
            node,
            &Request::Motion(motion::Request::Stop(motion::Stop { channel })),
            &Response::Motion(motion::Response::Accepted { operation: stop_operation }),
            207_200,
            &diagnostics,
        );
        assert_eq!(response, None, "the stop's own admission has room; nothing overrides it");

        // A client-style read of the unrelated operation, taken right
        // after the stop was accepted, must still see it running -- not a
        // same-instant guess that it stopped too.
        assert_eq!(
            core.operations().status(node, unrelated_operation).unwrap().state,
            OperationState::Active,
            "accepting a stop for a different operation must not disturb this one"
        );

        // The node's own authoritative completion arrives, exactly as the
        // soak's journal recorded it (normal PositionReached, no Stop ever
        // reaching this channel's motor).
        observe_operation_event(
            &mut core,
            node,
            Event {
                id: EventId(1),
                payload: EventPayload::Motion(motion::Event::PositionReached {
                    channel,
                    operation: unrelated_operation,
                    requested: motion::Target::Rotational { microdegrees: 0 },
                    actual: motion::Target::Rotational { microdegrees: 0 },
                    timing: motion::Timing::default(),
                }),
            },
            207_475,
        );
        assert_eq!(
            core.operations().status(node, unrelated_operation).unwrap().state,
            OperationState::Completed,
            "the node's own completion is authoritative and must be observable"
        );
    }

    #[test]
    fn move_relative_admission() {
        let node = NodeId(566_114);
        let channel = ChannelId(1);
        let operation = OperationId(1);
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        let diagnostics = Diagnostics::new();

        let response = observe_operation_response(
            &mut core,
            node,
            &Request::Motion(motion::Request::MoveRelative(motion::MoveRelative {
                channel,
                delta: motion::Delta::Rotational { microdegrees: 0 },
                settings: motion::Settings::default(),
            })),
            &Response::Motion(motion::Response::Accepted { operation }),
            1_000,
            &diagnostics,
        );
        assert_eq!(response, None, "the cache has room; nothing overrides it");

        assert_eq!(
            core.operations().status(node, operation),
            Some(OperationSnapshot { error: None, cleanup_error: None, id: operation, channel, state: OperationState::Active }),
            "a MoveRelative acceptance must populate the cache exactly like MoveTo/SetHome/Stop, \
             so the next status read is a cache hit instead of a live exchange"
        );
    }

    #[test]
    fn saturated_operation_untracked() {
        let node = NodeId(566_114);
        let channel = ChannelId(1);
        let mut core = GatewayCore::new(30_000);
        core.register(describe(node.0), 0).unwrap();
        let diagnostics = Diagnostics::new();

        // Saturate every tracked-operation slot with a still-active entry,
        // so the cache has no free slot and no terminal entry to evict.
        for id in 1..=u32::try_from(MAX_TRACKED_OPERATIONS).unwrap() {
            core.operations_mut().accepted(node, OperationId(id), channel, 0).unwrap();
        }

        let new_operation = OperationId(u32::try_from(MAX_TRACKED_OPERATIONS).unwrap() + 1);
        let response = observe_operation_response(
            &mut core,
            node,
            &Request::Motion(motion::Request::MoveTo(motion::MoveTo {
                channel,
                target: motion::Target::Rotational { microdegrees: 0 },
                settings: motion::Settings::default(),
            })),
            &Response::Motion(motion::Response::Accepted { operation: new_operation }),
            1_000,
            &diagnostics,
        );

        assert_eq!(response, Some(Error::Busy.into()), "a host must never be handed an operation id the saturated cache could not track");
        assert!(core.operations().status(node, new_operation).is_none(), "the untracked operation must not appear in the cache at all");
    }
}
