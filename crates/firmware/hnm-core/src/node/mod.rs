/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// The one dispatcher family every HNM node runs.

/* === Modules === */

pub mod ctx;
pub mod definition;
pub mod services;
pub mod storage;

/* === Re-Exports === */

pub use ctx::{ConfigCtx, Ctx, InvalidDocument, Origin};
pub use definition::{MAX_STATUS_ROWS, Node, Outcome, StatusRow, StatusRows, StatusValue};

/* === Imports === */

// Standard Library Imports
use core::marker::PhantomData;

// Third Party Imports
use embassy_futures::select::{Either, select};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, TrySendError},
};
use embassy_time::{Duration, Instant, Ticker};
use embedded_storage::nor_flash::NorFlash;

// Local Imports
use self::{
    ctx::ConfigSlot,
    services::{
        Services,
        binding::Binding,
        events::Events,
        peers::{self, Peers},
        replay::{Cache as ReplayCache, Lookup as ReplayLookup},
    },
    storage::Storage,
};
use crate::{
    boot::BootReport,
    diagnostics::Diagnostics,
    identity::Identity,
    mesh::coordinator,
    platform::Platform,
    ui::{self, LocalPage, NearbyPage, RequestActivity, RoundTripStats, Snapshot},
};
use hnm_protocol::{
    diagnostics::JournalEvent,
    error::Error,
    node::{self as wire_node, FIRMWARE_VERSION, NodeDescription, NodeId, NodeInfo, NodeStatus, NodeSummary, RequestId, SessionId},
    satellite::{MAX_SATELLITES, SatelliteReport},
    wire::{self, Event, PROTOCOL_VERSION, RequestEnvelope, Response, ResponseEnvelope},
};

/* === Constants === */

const TICK: Duration = Duration::from_millis(250);
const INPUT_CAPACITY: usize = 8;
const RESPONSE_CAPACITY: usize = 8;
const REPLAY_CAPACITY: usize = 6;
const FORWARD_CAPACITY: usize = 8;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug)]
pub struct Submission {
    pub origin: Origin,

    pub version: u8,

    pub request_id: RequestId,

    pub destination: NodeId,

    pub request: wire::Request,
}

#[derive(Clone, Debug)]
pub(crate) struct MeshReply {
    pub(crate) response: Response,
    pub(crate) event: Option<Event>,
}

#[derive(Clone, Copy)]
struct Forward {
    origin: Origin,
    request_id: RequestId,
}

struct Forwards {
    entries: [Option<Forward>; FORWARD_CAPACITY],
}

pub(crate) struct Shared<R: Node> {
    role: PhantomData<R>,
    identity: Identity,
    boot: BootReport,
    inputs: Channel<CriticalSectionRawMutex, Input, INPUT_CAPACITY>,
    usb_responses: Channel<CriticalSectionRawMutex, ResponseEnvelope, RESPONSE_CAPACITY>,
    mesh_replies: Channel<CriticalSectionRawMutex, MeshReply, 1>,
    diagnostics: Diagnostics,
    ui: ui::Channel,
}

struct Inner<R: Node, F, P> {
    role: R,
    storage: Storage<F>,
    platform: P,
    services: Services,
    replay: ReplayCache<REPLAY_CAPACITY>,
    config: ConfigSlot<R::Config>,
    coordinator: Option<coordinator::Handle>,
    forwards: Forwards,
    activity: Option<RequestActivity>,
    round_trip: Option<RoundTripStats>,
    binding: Binding,
}

pub struct Runtime<R: Node, F: NorFlash, P: Platform> {
    shared: Shared<R>,
    inner: Inner<R, F, P>,
}

pub struct Handle<R: Node + 'static> {
    shared: &'static Shared<R>,
}

pub struct Runner<R, F, P>
where
    R: Node + 'static,
    F: NorFlash + 'static,
    P: Platform + 'static,
{
    shared: &'static Shared<R>,
    inner: &'static mut Inner<R, F, P>,
}

/* === Enums === */

// `Remote(ResponseEnvelope)` now dwarfs the other variants: a v7 gateway
// roster page inlines up to `NODE_PAGE_SIZE` capability-bearing entries by
// value (see `hnm_protocol::wire::Response`'s doc comment). No allocator is
// available to `Box` it behind, and every variant already lives on this
// dispatcher's typed input channel regardless of which one is in play.
#[allow(clippy::large_enum_variant)]
enum Input {
    Request(Submission),
    Remote(ResponseEnvelope),
    Nearby(peers::Record),
    RoundTrip(RoundTripStats),
}

/* === Traits === */

/* === Implementations === */

impl Forwards {
    const fn new() -> Self {
        Self { entries: [None; FORWARD_CAPACITY] }
    }

    fn insert(&mut self, forward: Forward) -> Result<(), ()> {
        if self.entries.iter().flatten().any(|entry| entry.request_id == forward.request_id) {
            return Err(());
        }
        let slot = self.entries.iter_mut().find(|entry| entry.is_none()).ok_or(())?;
        *slot = Some(forward);
        Ok(())
    }

    fn remove(&mut self, request_id: RequestId) -> Option<Forward> {
        self.entries.iter_mut().find(|entry| entry.is_some_and(|forward| forward.request_id == request_id)).and_then(Option::take)
    }
}

impl<R: Node> Shared<R> {
    fn new(identity: Identity, boot: BootReport) -> Self {
        let diagnostics = Diagnostics::new();
        for report in boot.satellites() {
            diagnostics.record(JournalEvent::SatelliteState { name: report.name, state: report.state });
        }
        Self {
            role: PhantomData,
            identity,
            boot,
            inputs: Channel::new(),
            usb_responses: Channel::new(),
            mesh_replies: Channel::new(),
            diagnostics,
            ui: ui::Channel::new(),
        }
    }
}

impl<R, F, P> Runtime<R, F, P>
where
    R: Node,
    F: NorFlash,
    P: Platform,
{
    #[must_use]
    pub fn compose(identity: Identity, flash: F, platform: P, boot: BootReport, mut role: R) -> Self {
        let mut storage = Storage::new(flash);
        let mut config = ConfigSlot { current: None, pending: None };
        match storage.configuration().load::<R::Config>() {
            Ok(Some(document)) => {
                role.restore(&document);
                config.current = Some(document);
            }
            Ok(None) => {}
            Err(_) => defmt::warn!("configuration load failed"),
        }
        Self {
            shared: Shared::new(identity, boot),
            inner: Inner {
                role,
                storage,
                platform,
                services: Services { events: Events::new(), peers: Peers::new() },
                replay: ReplayCache::new(),
                config,
                coordinator: None,
                forwards: Forwards::new(),
                activity: None,
                round_trip: None,
                binding: Binding::new(),
            },
        }
    }

    #[must_use]
    pub fn forward_to(mut self, coordinator: coordinator::Handle) -> Self {
        self.inner.coordinator = Some(coordinator);
        self
    }

    pub fn split(&'static mut self) -> (Runner<R, F, P>, Handle<R>) {
        let shared: &'static Shared<R> = &self.shared;
        let inner: &'static mut Inner<R, F, P> = &mut self.inner;
        (Runner { shared, inner }, Handle { shared })
    }
}

impl<R: Node + 'static> Clone for Handle<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: Node + 'static> Copy for Handle<R> {}

impl<R: Node + 'static> Handle<R> {
    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.shared.identity.node
    }

    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.shared.identity.session
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &'static Diagnostics {
        &self.shared.diagnostics
    }

    #[must_use]
    pub fn ui(&self) -> Option<ui::Receiver> {
        let shared: &'static Shared<R> = self.shared;
        shared.ui.receiver()
    }

    pub async fn submit(&self, submission: Submission) {
        self.shared.inputs.send(Input::Request(submission)).await;
    }

    pub async fn next_usb_response(&self) -> ResponseEnvelope {
        self.shared.usb_responses.receive().await
    }

    pub(crate) async fn mesh_request(&self, submission: Submission) -> MeshReply {
        self.shared.inputs.send(Input::Request(submission)).await;
        self.shared.mesh_replies.receive().await
    }

    pub(crate) async fn remote_response(&self, envelope: ResponseEnvelope) {
        self.shared.inputs.send(Input::Remote(envelope)).await;
    }

    pub(crate) async fn observe_nearby(&self, record: peers::Record) {
        self.shared.inputs.send(Input::Nearby(record)).await;
    }

    pub(crate) async fn observe_round_trip(&self, stats: RoundTripStats) {
        self.shared.inputs.send(Input::RoundTrip(stats)).await;
    }
}

impl<R, F, P> Runner<R, F, P>
where
    R: Node + 'static,
    F: NorFlash + 'static,
    P: Platform + 'static,
{
    pub async fn run(mut self) -> ! {
        let mut ticker = Ticker::every(TICK);
        self.publish_ui();
        loop {
            match select(self.shared.inputs.receive(), ticker.next()).await {
                Either::First(Input::Request(submission)) => {
                    self.handle_submission(submission).await;
                }
                Either::First(Input::Remote(envelope)) => self.complete_remote(envelope),
                Either::First(Input::Nearby(record)) => {
                    self.inner.platform.led().communicating();
                    self.inner.services.peers.update(record);
                    self.publish_ui();
                }
                Either::First(Input::RoundTrip(stats)) => {
                    self.inner.round_trip = Some(stats);
                    self.publish_ui();
                }
                Either::Second(()) => self.handle_tick(),
            }
        }
    }

    async fn handle_submission(&mut self, mut submission: Submission) {
        self.shared.diagnostics.request_received();
        self.inner.activity = Some(RequestActivity::observed(submission.origin, submission.request_id, submission.destination));
        self.publish_ui();

        let origin = submission.origin;
        let request_id = submission.request_id;
        let now_ms = Instant::now().as_millis();

        // Version and origin gates come first. A rejected envelope's
        // acknowledgment facts are untrusted, so no pending event is
        // attached to (or compacted by) a gate rejection.
        if submission.version != PROTOCOL_VERSION {
            self.finish_request(origin, request_id, Error::UnsupportedVersion.into(), false).await;
            return;
        }
        if let Err(error) = self.inner.role.before_request(&mut submission.request) {
            self.finish_request(origin, request_id, error.into(), false).await;
            return;
        }
        if !origin_allows(origin, &submission.request) {
            self.finish_request(origin, request_id, Error::Unauthorized.into(), false).await;
            return;
        }

        // `None` means the request was forwarded and completes later through
        // its keyed remote response.
        if let Some(response) = self.admit(submission, now_ms).await {
            self.finish_request(origin, request_id, response, true).await;
        }
    }

    async fn admit(&mut self, submission: Submission, now_ms: u64) -> Option<Response> {
        self.inner.platform.led().communicating();
        if let Origin::Lora { peer, .. } = submission.origin {
            self.shared.diagnostics.record(JournalEvent::MeshRequestReceived { peer, request_id: submission.request_id });
            self.inner.services.peers.observe(peer, now_ms);
        }

        // Binding precedes replay: retrying any request from the bound
        // gateway/session is still fresh traffic and must refresh its silence
        // deadline, while the replay cache below still prevents a repeated
        // physical effect.
        if let Err(error) = admit_write_binding(&mut self.inner.binding, submission.origin, &submission.request, now_ms) {
            return Some(error.into());
        }

        // Replay or reject duplicate non-broadcast mesh requests before any
        // repeated physical effect.
        let replay_key = match submission.origin {
            Origin::Lora { peer, session, broadcast: false, .. } => Some((peer, session)),
            Origin::Lora { .. } | Origin::Usb => None,
        };
        if let Some((peer, session)) = replay_key {
            match self.inner.replay.lookup(peer, session, submission.request_id, &submission.request) {
                ReplayLookup::Replay(response) => return Some(response.clone()),
                ReplayLookup::Conflict => {
                    return Some(Error::DuplicateRequestConflict.into());
                }
                ReplayLookup::Miss => {}
            }
        }

        // Envelope-level routing. A request whose destination is a remote
        // node — and a mesh-brokerage request on any destination — belongs to
        // the attached coordinator. One consistent foreign-node rule applies
        // to everything else, ReadDiagnostics/ReadJournal/OperationStatus
        // included: destination != self and not forwardable => NotFound.
        let local = submission.destination == self.shared.identity.node || submission.destination == NodeId::BROADCAST;
        let brokered = matches!(submission.request, wire::Request::Gateway(_)) && self.inner.coordinator.is_some();
        if !local || brokered {
            if !matches!(submission.origin, Origin::Usb) {
                return Some(Error::NotFound.into());
            }
            let Some(coordinator) = self.inner.coordinator else {
                return Some(Error::NotFound.into());
            };
            if self.inner.forwards.insert(Forward { origin: submission.origin, request_id: submission.request_id }).is_err() {
                return Some(Error::Busy.into());
            }
            let envelope = RequestEnvelope {
                version: PROTOCOL_VERSION,
                request_id: submission.request_id,
                destination: submission.destination,
                request: submission.request,
            };
            if coordinator.try_forward(envelope).is_err() {
                let _ = self.inner.forwards.remove(submission.request_id);
                return Some(Error::Busy.into());
            }
            return None;
        }

        let request_id = submission.request_id;
        let replayable = replay_key.map(|_| submission.request.clone());
        let response = self.execute(submission, now_ms).await;
        if let (Some((peer, session)), Some(request)) = (replay_key, replayable) {
            self.inner.replay.insert(peer, session, request_id, request, response.clone());
        }
        Some(response)
    }

    async fn execute(&mut self, submission: Submission, now_ms: u64) -> Response {
        match submission.request {
            wire::Request::Node(request)
                if !matches!(request, wire_node::Request::ConfigureAuxGpio(_) | wire_node::Request::PulseAuxGpio(_)) =>
            {
                self.answer_node(request)
            }
            other => {
                let Ok(request) = R::Request::try_from(other) else {
                    return Error::Unsupported.into();
                };
                let inner = &mut *self.inner;
                let mut cx = Ctx {
                    node: self.shared.identity.node,
                    origin: Some(submission.origin),
                    request_id: Some(submission.request_id),
                    now_ms,
                    config: &mut inner.config,
                    diagnostics: &self.shared.diagnostics,
                    role: PhantomData,
                };
                let outcome = inner.role.handle_request(&mut cx, request).await;
                let mut response = outcome.response;
                // A staged config store completes before the response is sent.
                if !self.persist_staged_config() {
                    response = Error::Unavailable.into();
                }
                response
            }
        }
    }

    fn answer_node(&mut self, request: wire_node::Request) -> Response {
        match request {
            wire_node::Request::Describe(_) => {
                wire_node::Response::Description(node_description::<R>(self.shared.identity.node, self.shared.boot.wire_reports())).into()
            }
            wire_node::Request::Status(_) => wire_node::Response::Status(NodeStatus {
                info: node_info::<R>(self.shared.identity.node),
                auxiliary_gpio: self.inner.platform.auxiliary_gpio_modes(),
                auxiliary_gpio_access: self.inner.platform.auxiliary_gpio_access(),
            })
            .into(),
            wire_node::Request::Identify(_) => {
                if self.inner.platform.identify() {
                    wire_node::Response::Identified.into()
                } else {
                    Error::Busy.into()
                }
            }
            wire_node::Request::ReadDiagnostics(_) => {
                wire_node::Response::Diagnostics(self.shared.diagnostics.snapshot(self.shared.identity.node)).into()
            }
            wire_node::Request::ReadJournal(request) => {
                wire_node::Response::Journal(self.shared.diagnostics.page(request.after_sequence)).into()
            }
            wire_node::Request::CancelOperation(request) => match self.inner.role.cancel_operation(request.operation) {
                Ok(()) => wire_node::Response::OperationCancelled.into(),
                Err(error) => error.into(),
            },
            wire_node::Request::OperationStatus(request) => self
                .inner
                .role
                .operation_status(request.operation)
                .map_or_else(|| Error::NotFound.into(), |snapshot| wire_node::Response::OperationStatus(snapshot).into()),
            wire_node::Request::ConfigureAuxGpio(_) | wire_node::Request::PulseAuxGpio(_) => Error::Unsupported.into(),
            wire_node::Request::Restart(request) => {
                self.inner.platform.restart(request.bootloader);
                wire_node::Response::RestartScheduled { bootloader: request.bootloader }.into()
            }
            wire_node::Request::Poll(_) => wire_node::Response::Polled.into(),
        }
    }

    async fn finish_request(&mut self, origin: Origin, request_id: RequestId, mut response: Response, attach_event: bool) {
        self.inner.role.after_response(&mut response);
        let succeeded = response.is_success();
        self.shared.diagnostics.request_completed(succeeded);
        if let Some(activity) = &mut self.inner.activity {
            activity.complete(origin, request_id, succeeded);
        }
        match origin {
            Origin::Usb => {
                self.push_usb_response(ResponseEnvelope { version: PROTOCOL_VERSION, request_id, response });
            }
            Origin::Lora { peer, acknowledged_session, acknowledged_event, .. } => {
                let event = attach_event
                    .then(|| {
                        self.inner.services.events.next_after(peer, self.shared.identity.session, acknowledged_session, acknowledged_event)
                    })
                    .flatten();
                if let Some(event) = event {
                    self.shared.diagnostics.record(JournalEvent::CapabilityEventReturned { peer, request_id, event });
                }
                self.shared.mesh_replies.send(MeshReply { response, event }).await;
            }
        }
        self.publish_ui();
    }

    fn complete_remote(&mut self, envelope: ResponseEnvelope) {
        let Some(forward) = self.inner.forwards.remove(envelope.request_id) else {
            return;
        };
        let succeeded = envelope.response.is_success();
        self.shared.diagnostics.request_completed(succeeded);
        if let Some(activity) = &mut self.inner.activity {
            activity.complete(forward.origin, envelope.request_id, succeeded);
        }
        if matches!(forward.origin, Origin::Usb) {
            self.push_usb_response(envelope);
        }
        self.publish_ui();
    }

    fn push_usb_response(&mut self, response: ResponseEnvelope) {
        if let Err(TrySendError::Full(response)) = self.shared.usb_responses.try_send(response) {
            defmt::warn!("USB response channel full; dropping oldest queued response");
            let _ = self.shared.usb_responses.try_receive();
            let _ = self.shared.usb_responses.try_send(response);
        }
    }

    fn handle_tick(&mut self) {
        let now_ms = Instant::now().as_millis();
        {
            let inner = &mut *self.inner;
            let mut cx = Ctx {
                node: self.shared.identity.node,
                origin: None,
                request_id: None,
                now_ms,
                config: &mut inner.config,
                diagnostics: &self.shared.diagnostics,
                role: PhantomData,
            };
            inner.role.tick(&mut cx);
        }
        if !self.persist_staged_config() {
            defmt::warn!("configuration store failed");
        }
        let expired = self.inner.services.peers.expire(now_ms);
        if expired {
            self.publish_ui();
        }
    }

    fn persist_staged_config(&mut self) -> bool {
        let inner = &mut *self.inner;
        let Some(document) = inner.config.pending.take() else {
            return true;
        };
        inner.storage.configuration().store(document).is_ok()
    }

    fn publish_ui(&mut self) {
        let inner = &mut *self.inner;
        let mut local = LocalPage::new(self.shared.identity.node, R::NODE_TYPE.definition().label);

        let nearby_count = inner.services.peers.iter().count();
        let mut rows = StatusRows::new();
        if let Some(stats) = inner.round_trip {
            rows.push("RTT Last / Mean", StatusValue::RoundTrip(stats));
        }
        rows.push("Nearby Devices", StatusValue::Count(u32::try_from(nearby_count).unwrap_or(u32::MAX)));
        inner.role.status(&mut rows);
        local.rows = rows;

        let mut snapshot = Snapshot::new(local);
        snapshot.activity = inner.activity;
        for record in inner.services.peers.iter() {
            snapshot.upsert_nearby(NearbyPage { id: record.id, label: record.label.clone(), last_seen_ms: record.last_seen_ms });
        }
        self.shared.ui.sender().send(snapshot);
    }
}

/* === Public Functions === */

pub(crate) fn node_info<R: Node>(node: NodeId) -> NodeInfo {
    NodeInfo { node_type: R::NODE_TYPE, id: node, firmware: FIRMWARE_VERSION }
}

pub(crate) fn node_summary<R: Node>(node: NodeId) -> NodeSummary {
    NodeSummary {
        info: node_info::<R>(node),
        // The node answering for itself is by definition not silent.
        silent_for_s: 0,
    }
}

/* === Private Functions === */

const fn origin_allows(origin: Origin, request: &wire::Request) -> bool {
    struct Access {
        usb: bool,
        mesh: bool,
    }
    let access = match request {
        wire::Request::Gateway(_) => Access { usb: true, mesh: false },
        wire::Request::Node(_) | wire::Request::Motion(_) => Access { usb: true, mesh: true },
    };
    match origin {
        Origin::Usb => access.usb,
        Origin::Lora { .. } => access.mesh,
    }
}

const fn is_write(request: &wire::Request) -> bool {
    matches!(
        request,
        wire::Request::Motion(_)
            | wire::Request::Node(
                wire_node::Request::ConfigureAuxGpio(_)
                    | wire_node::Request::PulseAuxGpio(_)
                    | wire_node::Request::Restart(_)
                    | wire_node::Request::CancelOperation(_)
            )
    )
}

fn admit_write_binding(binding: &mut Binding, origin: Origin, request: &wire::Request, now_ms: u64) -> Result<(), Error> {
    let Origin::Lora { peer, session, .. } = origin else {
        return Ok(());
    };
    binding.admit(peer, session, is_write(request), now_ms).map_err(|gateway| Error::BoundElsewhere { gateway })
}

fn node_description<R: Node>(node: NodeId, satellites: heapless::Vec<SatelliteReport, MAX_SATELLITES>) -> NodeDescription {
    NodeDescription { info: node_info::<R>(node), satellites }
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::motion;
    extern crate std;

    use hnm_protocol::node::{AuxGpioId, AuxGpioMode, EventId};

    use super::*;

    const NODE: NodeId = NodeId(566_114);
    const GATEWAY: NodeId = NodeId(273_282);
    const OTHER_GATEWAY: NodeId = NodeId(654_321);

    const fn lora() -> Origin {
        lora_from(GATEWAY, SessionId(1))
    }

    const fn lora_from(peer: NodeId, session: SessionId) -> Origin {
        Origin::Lora { peer, session, acknowledged_session: SessionId::UNKNOWN, acknowledged_event: EventId(0), broadcast: false }
    }

    fn gpio_write() -> wire::Request {
        wire::Request::Node(wire_node::Request::ConfigureAuxGpio(wire_node::ConfigureAuxGpio {
            gpio: AuxGpioId(1),
            mode: AuxGpioMode::DriveHigh,
        }))
    }

    fn describe_read() -> wire::Request {
        wire::Request::Node(wire_node::Request::Describe(wire_node::Describe))
    }

    #[test]
    fn dispatcher_rejects_requests() {
        let poll = wire::Request::Node(wire_node::Request::Poll(wire_node::Poll));
        let diagnostics = wire::Request::Node(wire_node::Request::ReadDiagnostics(wire_node::ReadDiagnostics));
        let restart = wire::Request::Node(wire_node::Request::Restart(wire_node::Restart { bootloader: None }));
        let nodes = wire::Request::Gateway(hnm_protocol::gateway::Request::Nodes(hnm_protocol::gateway::Nodes { after: None }));
        let describe = wire::Request::Node(wire_node::Request::Describe(wire_node::Describe));

        assert!(origin_allows(Origin::Usb, &poll));
        assert!(origin_allows(lora(), &poll));
        assert!(origin_allows(lora(), &diagnostics));
        assert!(origin_allows(lora(), &restart));
        assert!(!origin_allows(lora(), &nodes));
        assert!(origin_allows(Origin::Usb, &describe));
        assert!(origin_allows(lora(), &describe));
    }

    #[test]
    fn dispatcher_classifies_write() {
        let restart = wire::Request::Node(wire_node::Request::Restart(wire_node::Restart { bootloader: None }));
        let configure = wire::Request::Motion(motion::Request::Configure(motion::Configure {
            configuration: motion::Configuration { steppers: heapless::Vec::new() },
        }));

        assert!(is_write(&gpio_write()));
        assert!(is_write(&restart));
        assert!(is_write(&configure));
        assert!(!is_write(&describe_read()));
        assert!(!is_write(&wire::Request::Node(wire_node::Request::Identify(wire_node::Identify,))));
        assert!(!is_write(&wire::Request::Node(wire_node::Request::Status(wire_node::Status,))));
    }

    #[test]
    fn dispatcher_binding_expires() {
        let mut binding = Binding::new();

        assert_eq!(admit_write_binding(&mut binding, lora(), &gpio_write(), 0), Ok(()));
        assert_eq!(
            admit_write_binding(&mut binding, lora_from(OTHER_GATEWAY, SessionId(8)), &gpio_write(), 1,),
            Err(Error::BoundElsewhere { gateway: GATEWAY })
        );
        assert_eq!(
            admit_write_binding(&mut binding, lora_from(OTHER_GATEWAY, SessionId(8)), &describe_read(), 2,),
            Ok(()),
            "reads from another gateway remain open while the write binding is held"
        );
        assert_eq!(
            admit_write_binding(&mut binding, lora(), &describe_read(), services::binding::BINDING_TIMEOUT_MS - 1,),
            Ok(()),
            "even a replayable read from the holder is admitted before replay lookup and refreshes"
        );
        assert_eq!(
            admit_write_binding(&mut binding, lora_from(OTHER_GATEWAY, SessionId(8)), &gpio_write(), services::binding::BINDING_TIMEOUT_MS,),
            Err(Error::BoundElsewhere { gateway: GATEWAY }),
            "the holder's just-refreshed deadline remains authoritative"
        );
        assert_eq!(
            admit_write_binding(
                &mut binding,
                lora_from(OTHER_GATEWAY, SessionId(8)),
                &gpio_write(),
                2 * services::binding::BINDING_TIMEOUT_MS - 1,
            ),
            Ok(()),
            "the next radio write binds after 30 seconds of holder silence"
        );
    }

    #[test]
    fn dispatcher_binding_bypass() {
        let mut binding = Binding::new();
        admit_write_binding(&mut binding, lora(), &gpio_write(), 0).unwrap();

        assert_eq!(
            admit_write_binding(&mut binding, lora_from(GATEWAY, SessionId(2)), &gpio_write(), 1,),
            Ok(()),
            "a different session of the same gateway takes over immediately on a write"
        );
        assert_eq!(
            admit_write_binding(&mut binding, Origin::Usb, &gpio_write(), 2),
            Ok(()),
            "physical USB writes bypass the radio binding"
        );
        assert_eq!(
            admit_write_binding(&mut binding, lora_from(OTHER_GATEWAY, SessionId(3)), &gpio_write(), 3,),
            Err(Error::BoundElsewhere { gateway: GATEWAY }),
            "a USB write neither establishes nor clears the radio binding"
        );
    }

    #[test]
    fn forwards_reject_ids() {
        let mut forwards = Forwards::new();
        let forward = Forward { origin: Origin::Usb, request_id: RequestId(7) };
        assert!(forwards.insert(forward).is_ok());
        assert!(forwards.insert(forward).is_err());
        assert!(forwards.remove(RequestId(7)).is_some());
        assert!(forwards.remove(RequestId(7)).is_none());
    }

    #[test]
    fn describe_carries_report() {
        use hnm_protocol::satellite::{SatelliteName, SatelliteReport, SatelliteState};

        struct TestRole;

        impl Node for TestRole {
            type Request = wire::Request;

            type Config = ();

            const NODE_TYPE: hnm_protocol::nodes::NodeType = hnm_protocol::nodes::NodeType::Sidecar;

            fn handle_request(&mut self, _cx: &mut Ctx<'_, Self>, _request: Self::Request) -> impl core::future::Future<Output = Outcome> {
                core::future::ready(Outcome::reply(Error::Unsupported))
            }
        }

        let mut satellites = heapless::Vec::new();
        satellites.push(SatelliteReport { name: SatelliteName::MotorWing, state: SatelliteState::Ready }).unwrap();
        let description = node_description::<TestRole>(NODE, satellites.clone());
        assert_eq!(description.satellites, satellites);
    }

    fn poll_bounded<F: core::future::Future>(future: F, steps: usize) -> Option<F::Output> {
        use core::{
            pin::pin,
            task::{Context, Poll, Waker},
        };

        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..steps {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return Some(output);
            }
        }
        None
    }

    struct WedgeRole;

    impl Node for WedgeRole {
        type Request = wire::Request;

        type Config = ();

        const NODE_TYPE: hnm_protocol::nodes::NodeType = hnm_protocol::nodes::NodeType::Sidecar;

        fn handle_request(&mut self, _cx: &mut Ctx<'_, Self>, _request: Self::Request) -> impl core::future::Future<Output = Outcome> {
            core::future::ready(Outcome::reply(Error::Unsupported))
        }
    }

    struct WedgeFlash;

    impl embedded_storage::nor_flash::ErrorType for WedgeFlash {
        type Error = embedded_storage::nor_flash::NorFlashErrorKind;
    }

    impl embedded_storage::nor_flash::ReadNorFlash for WedgeFlash {
        const READ_SIZE: usize = 1;

        fn read(&mut self, _offset: u32, _bytes: &mut [u8]) -> Result<(), Self::Error> {
            unreachable!("a Config = () role never touches flash")
        }

        fn capacity(&self) -> usize {
            0
        }
    }

    impl embedded_storage::nor_flash::NorFlash for WedgeFlash {
        const WRITE_SIZE: usize = 1;
        const ERASE_SIZE: usize = 1;

        fn write(&mut self, _offset: u32, _bytes: &[u8]) -> Result<(), Self::Error> {
            unreachable!("a Config = () role never touches flash")
        }

        fn erase(&mut self, _from: u32, _to: u32) -> Result<(), Self::Error> {
            unreachable!("a Config = () role never touches flash")
        }
    }

    struct WedgePlatform;

    impl Platform for WedgePlatform {
        fn auxiliary_gpio_access(&self) -> [wire_node::AuxGpioAccess; wire_node::AUX_GPIO_COUNT] {
            [wire_node::AuxGpioAccess::PushPull; wire_node::AUX_GPIO_COUNT]
        }

        fn auxiliary_gpio_modes(&self) -> [wire_node::AuxGpioMode; wire_node::AUX_GPIO_COUNT] {
            unreachable!("test drives only universal Node::Poll requests")
        }

        fn restart(&mut self, _bootloader: Option<wire_node::Bootloader>) {
            unreachable!("test drives only universal Node::Poll requests")
        }

        fn led(&self) -> crate::platform::led::Handle {
            static SIGNALS: crate::platform::led::Signals = crate::platform::led::Signals::new();
            SIGNALS.split().0
        }
    }

    struct ReplayNode {
        writes: usize,
        refusal: Option<wire_node::AuxGpioError>,
    }

    struct ReplayPlatform;

    impl Node for ReplayNode {
        type Request = wire::Request;

        type Config = ();
        const NODE_TYPE: hnm_protocol::nodes::NodeType = WedgeRole::NODE_TYPE;
        fn handle_request(&mut self, _cx: &mut Ctx<'_, Self>, request: Self::Request) -> impl core::future::Future<Output = Outcome> {
            if let Some(reason) = self.refusal {
                return core::future::ready(Outcome::reply(Error::AuxGpio { gpio: AuxGpioId(1), reason }));
            }
            self.writes += 1;
            core::future::ready(match request {
                wire::Request::Node(wire_node::Request::ConfigureAuxGpio(request)) => {
                    Outcome::reply(wire_node::Response::AuxGpioConfigured { gpio: request.gpio, mode: request.mode })
                }
                wire::Request::Node(wire_node::Request::PulseAuxGpio(request)) => {
                    Outcome::reply(wire_node::Response::AuxGpioPulseScheduled {
                        gpio: request.gpio,
                        duration_ms: request.duration_ms,
                        operation: hnm_protocol::node::OperationId(1),
                    })
                }
                _ => Outcome::reply(Error::Unsupported),
            })
        }
    }

    impl Platform for ReplayPlatform {
        fn auxiliary_gpio_access(&self) -> [wire_node::AuxGpioAccess; wire_node::AUX_GPIO_COUNT] {
            [wire_node::AuxGpioAccess::PushPull; wire_node::AUX_GPIO_COUNT]
        }

        fn auxiliary_gpio_modes(&self) -> [wire_node::AuxGpioMode; wire_node::AUX_GPIO_COUNT] {
            [wire_node::AuxGpioMode::HighImpedance; wire_node::AUX_GPIO_COUNT]
        }

        fn restart(&mut self, _bootloader: Option<wire_node::Bootloader>) {}

        fn led(&self) -> crate::platform::led::Handle {
            static SIGNALS: crate::platform::led::Signals = crate::platform::led::Signals::new();
            SIGNALS.split().0
        }
    }

    fn lora_gpio_write(peer: NodeId, session: SessionId, request_id: RequestId) -> Submission {
        Submission { origin: lora_from(peer, session), version: PROTOCOL_VERSION, request_id, destination: NODE, request: gpio_write() }
    }

    fn lora_gpio_pulse(peer: NodeId, session: SessionId, request_id: RequestId) -> Submission {
        Submission {
            request: wire_node::PulseAuxGpio { gpio: AuxGpioId(1), duration_ms: 1 }.into(),
            ..lora_gpio_write(peer, session, request_id)
        }
    }

    #[test]
    fn gpio_refusal_reason() {
        let node = std::boxed::Box::leak(std::boxed::Box::new(Runtime::compose(
            Identity { node: NODE, session: SessionId(1) },
            WedgeFlash,
            ReplayPlatform,
            BootReport::default(),
            ReplayNode { writes: 0, refusal: Some(wire_node::AuxGpioError::Reserved) },
        )));
        let (mut runner, _handle) = node.split();
        let response = poll_bounded(runner.admit(lora_gpio_write(GATEWAY, SessionId(1), RequestId(1)), 0), 10_000).unwrap();
        assert_eq!(response, Some(Error::AuxGpio { gpio: AuxGpioId(1), reason: wire_node::AuxGpioError::Reserved }.into()));
        assert_eq!(runner.inner.role.writes, 0);
    }

    #[test]
    fn replayed_holder_effect() {
        const STEPS: usize = 10_000;
        const REQUEST: RequestId = RequestId(41);
        let identity = Identity { node: NODE, session: SessionId(1) };
        let node: &'static mut Runtime<ReplayNode, WedgeFlash, ReplayPlatform> = std::boxed::Box::leak(std::boxed::Box::new(
            Runtime::compose(identity, WedgeFlash, ReplayPlatform, BootReport::default(), ReplayNode { writes: 0, refusal: None }),
        ));
        let (mut runner, _handle) = node.split();

        let first =
            poll_bounded(runner.admit(lora_gpio_write(GATEWAY, SessionId(1), REQUEST), 0), STEPS).expect("the first write should complete");
        assert!(first.is_some());
        assert_eq!(runner.inner.role.writes, 1);

        let replayed =
            poll_bounded(runner.admit(lora_gpio_write(GATEWAY, SessionId(1), REQUEST), services::binding::BINDING_TIMEOUT_MS - 1), STEPS)
                .expect("the replay should complete");
        assert_eq!(replayed, first, "the cached response must be replayed");
        assert_eq!(runner.inner.role.writes, 1, "replay must not repeat the platform mutation");

        let rejected = poll_bounded(
            runner.admit(lora_gpio_write(OTHER_GATEWAY, SessionId(8), RequestId(42)), services::binding::BINDING_TIMEOUT_MS),
            STEPS,
        )
        .expect("the competing write should complete");
        assert_eq!(
            rejected,
            Some(Error::BoundElsewhere { gateway: GATEWAY }.into()),
            "the replay refreshed the holder before returning its cached response"
        );
        assert_eq!(runner.inner.role.writes, 1);
    }

    #[test]
    fn replayed_pulse_rescheduling() {
        const STEPS: usize = 10_000;
        const REQUEST: RequestId = RequestId(41);
        let identity = Identity { node: NODE, session: SessionId(1) };
        let node: &'static mut Runtime<ReplayNode, WedgeFlash, ReplayPlatform> = std::boxed::Box::leak(std::boxed::Box::new(
            Runtime::compose(identity, WedgeFlash, ReplayPlatform, BootReport::default(), ReplayNode { writes: 0, refusal: None }),
        ));
        let (mut runner, _handle) = node.split();

        let first =
            poll_bounded(runner.admit(lora_gpio_pulse(GATEWAY, SessionId(1), REQUEST), 0), STEPS).expect("the first write should complete");
        assert!(first.is_some());
        assert_eq!(runner.inner.role.writes, 1);

        let replayed =
            poll_bounded(runner.admit(lora_gpio_pulse(GATEWAY, SessionId(1), REQUEST), services::binding::BINDING_TIMEOUT_MS - 1), STEPS)
                .expect("the replay should complete");
        assert_eq!(replayed, first, "the cached response must be replayed");
        assert_eq!(runner.inner.role.writes, 1, "replay must not repeat the platform mutation");

        let rejected = poll_bounded(
            runner.admit(lora_gpio_pulse(OTHER_GATEWAY, SessionId(8), RequestId(42)), services::binding::BINDING_TIMEOUT_MS),
            STEPS,
        )
        .expect("the competing write should complete");
        assert_eq!(
            rejected,
            Some(Error::BoundElsewhere { gateway: GATEWAY }.into()),
            "the replay refreshed the holder before returning its cached response"
        );
        assert_eq!(runner.inner.role.writes, 1);
    }

    fn usb_poll(request_id: u32) -> Submission {
        Submission {
            origin: Origin::Usb,
            version: PROTOCOL_VERSION,
            request_id: RequestId(request_id),
            destination: NODE,
            request: wire::Request::Node(wire_node::Request::Poll(wire_node::Poll)),
        }
    }

    #[test]
    fn stalled_usb_request() {
        const STEPS: usize = 10_000;

        let identity = Identity { node: NODE, session: SessionId(1) };
        let node: &'static mut Runtime<WedgeRole, WedgeFlash, WedgePlatform> = std::boxed::Box::leak(std::boxed::Box::new(
            Runtime::compose(identity, WedgeFlash, WedgePlatform, BootReport::default(), WedgeRole),
        ));
        let (mut runner, _handle) = node.split();

        // Fill the bounded USB response channel to capacity. Each of these
        // completes immediately because room exists when it is sent -- the
        // writer resident is modeled as permanently stalled (nothing ever
        // drains `usb_responses` in this test) but that alone does not yet
        // reproduce the defect.
        for request_id in 1..=u32::try_from(RESPONSE_CAPACITY).unwrap() {
            let completed = poll_bounded(runner.handle_submission(usb_poll(request_id)), STEPS);
            assert!(completed.is_some(), "submission {request_id} should admit while the channel has room");
        }

        // One more USB-origin completion: this is the "next completion"
        // from the defect report that blocks forever inside
        // `finish_request`'s blocking send because nothing ever drains the
        // channel. On unfixed code this poll never reaches `Ready` within
        // the bound.
        let overflow_id = u32::try_from(RESPONSE_CAPACITY).unwrap() + 1;
        let overflowed = poll_bounded(runner.handle_submission(usb_poll(overflow_id)), STEPS);
        assert!(overflowed.is_some(), "a full USB response channel must never block the dispatcher's single owner loop");

        // The dispatcher's single owner loop must still be live *right now*,
        // with `usb_responses` still completely full (not yet drained
        // below): a radio-origin submission -- which never touches
        // `usb_responses` -- must complete and deliver its correlated
        // reply while the USB writer is still (modeled as) stalled.
        let lora_submission = Submission {
            origin: lora(),
            version: PROTOCOL_VERSION,
            request_id: RequestId(9000),
            destination: NODE,
            request: wire::Request::Node(wire_node::Request::Poll(wire_node::Poll)),
        };
        let completed = poll_bounded(runner.handle_submission(lora_submission), STEPS);
        assert!(completed.is_some(), "a radio-origin submission must complete even while the USB writer is stalled");
        let reply = runner.shared.mesh_replies.try_receive().expect("the radio-origin submission must deliver its correlated reply");
        assert!(matches!(reply.response, Response::Node(wire_node::Response::Polled)));

        // Drop-oldest: the newest completion lands, and the oldest queued
        // response was evicted to admit it. `usb_responses` is untouched by
        // the Lora-origin submission above (its reply went to
        // `mesh_replies`), so it still holds exactly what the USB-origin
        // fill and overflow left behind.
        let mut retained = heapless::Vec::<RequestId, RESPONSE_CAPACITY>::new();
        while let Ok(envelope) = runner.shared.usb_responses.try_receive() {
            retained.push(envelope.request_id).unwrap();
        }
        assert_eq!(retained.len(), RESPONSE_CAPACITY);
        assert_eq!(retained[0], RequestId(2), "the oldest queued response (1) must have been dropped");
        assert_eq!(retained[RESPONSE_CAPACITY - 1], RequestId(overflow_id), "the newest completion must still land");
    }
    struct MiddlewareNode {
        writes: usize,
        responses: usize,
        gateway_request: bool,
    }

    impl Node for MiddlewareNode {
        type Request = wire::Request;
        type Config = ();
        const NODE_TYPE: hnm_protocol::nodes::NodeType = WedgeRole::NODE_TYPE;

        fn before_request(&mut self, request: &mut wire::Request) -> Result<(), Error> {
            if matches!(request, wire::Request::Node(wire_node::Request::Poll(_))) {
                *request = if self.gateway_request { hnm_protocol::gateway::Nodes { after: None }.into() } else { gpio_write() };
            }
            Ok(())
        }

        fn after_response(&mut self, _response: &mut Response) {
            self.responses += 1;
        }

        fn handle_request(&mut self, _cx: &mut Ctx<'_, Self>, _request: Self::Request) -> impl core::future::Future<Output = Outcome> {
            self.writes += 1;
            core::future::ready(Outcome::reply(wire_node::Response::Polled))
        }
    }

    #[test]
    fn middleware_cannot_gates() {
        let node = std::boxed::Box::leak(std::boxed::Box::new(Runtime::compose(
            Identity { node: NODE, session: SessionId(1) },
            WedgeFlash,
            ReplayPlatform,
            BootReport::default(),
            MiddlewareNode { writes: 0, responses: 0, gateway_request: false },
        )));
        let (mut runner, _handle) = node.split();
        assert!(poll_bounded(runner.handle_submission(lora_gpio_write(GATEWAY, SessionId(1), RequestId(1))), 10_000).is_some());
        runner.shared.mesh_replies.try_receive().unwrap();
        assert_eq!(runner.inner.role.writes, 1);

        let read = Submission { request: wire_node::Poll.into(), ..lora_gpio_write(NodeId(GATEWAY.0 + 1), SessionId(2), RequestId(2)) };
        assert!(poll_bounded(runner.handle_submission(read), 10_000).is_some());
        assert_eq!(runner.shared.mesh_replies.try_receive().unwrap().response, Error::BoundElsewhere { gateway: GATEWAY }.into());
        assert_eq!(runner.inner.role.writes, 1);

        runner.inner.role.gateway_request = true;
        let read = Submission { request: wire_node::Poll.into(), ..lora_gpio_write(GATEWAY, SessionId(1), RequestId(3)) };
        assert!(poll_bounded(runner.handle_submission(read), 10_000).is_some());
        assert_eq!(runner.shared.mesh_replies.try_receive().unwrap().response, Error::Unauthorized.into());
        assert_eq!(runner.inner.role.writes, 1);
        assert_eq!(runner.inner.role.responses, 3);
    }
}
