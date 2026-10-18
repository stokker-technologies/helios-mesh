/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Fixed-capacity coordinator state and scheduling.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm_protocol::{
    gateway::{NODE_PAGE_SIZE, NodePage},
    motion,
    node::{
        self as wire_node, ChannelId, EventId, NodeDescription, NodeId, NodeSummary, OperationId, OperationSnapshot, OperationState,
        RequestId, SessionId,
    },
    wire::{Event, Request},
};

/* === Constants === */

pub const MAX_MANAGED_NODES: usize = 16;
pub const MAX_QUEUED_REQUESTS: usize = 32;
pub const MAX_TRACKED_OPERATIONS: usize = MAX_MANAGED_NODES * 2 * 2;
pub const DISCOVERY_SEARCH_INTERVAL_MS: u64 = 2_000;
pub const DISCOVERY_SETTLED_INTERVAL_MS: u64 = 60_000;
pub const DISCOVERY_SETTLE_WINDOW_MS: u64 = 30_000;
pub const DISCOVERY_COLLECTION_WINDOW_MS: u64 = 650;
pub const CONSECUTIVE_FAILURE_EVICTION_THRESHOLD: u8 = 8;
pub const EVICTION_SILENCE_MS: u64 = 240_000;
pub const RECONCILE_AFTER_MS: u64 = 2_000;
const RESERVED_EMERGENCY_SLOTS: usize = 2;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrackedOperation {
    node: NodeId,
    status: OperationSnapshot,
    updated_ms: u64,
}

pub struct OperationCache<const OPERATIONS: usize = MAX_TRACKED_OPERATIONS> {
    operations: [Option<TrackedOperation>; OPERATIONS],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledRequest {
    pub node: NodeId,

    pub request: Request,

    pub priority: Priority,

    pub ready_at_ms: u64,

    pub deadline_ms: u64,

    pub reply: Option<RequestId>,

    order: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRecord {
    pub description: NodeDescription,

    pub session: SessionId,

    pub acknowledged_event: EventId,

    pub last_seen_ms: u64,

    pub maintenance_due_ms: u64,

    pub consecutive_failures: u8,

    next_request_id: RequestId,
    last_served_order: u32,
}

struct DiscoverySchedule {
    due_ms: u64,
    settle_ms: u64,
}

pub struct GatewayCore<const NODES: usize = MAX_MANAGED_NODES, const QUEUED: usize = MAX_QUEUED_REQUESTS> {
    nodes: [Option<NodeRecord>; NODES],
    requests: [Option<ScheduledRequest>; QUEUED],
    operations: OperationCache,
    discovery: DiscoverySchedule,
    maintenance_interval_ms: u64,
    enqueue_order: u32,
    service_order: u32,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Priority {
    Emergency,

    Interactive,

    Operation,

    Maintenance,

    Discovery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("node registry is full")]
    RegistryFull,

    #[error("node is not registered")]
    UnknownNode,

    #[error("request queue is full")]
    QueueFull,

    #[error("operation registry is full")]
    OperationRegistryFull,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureOutcome {
    Retained,

    Evicted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryOutcome {
    Enrolled,

    LivenessRefreshed,
}

/* === Traits === */

/* === Implementations === */

impl<const OPERATIONS: usize> OperationCache<OPERATIONS> {
    #[must_use]
    pub const fn new() -> Self {
        Self { operations: [None; OPERATIONS] }
    }

    pub fn accepted(&mut self, node: NodeId, operation: OperationId, channel: ChannelId, now_ms: u64) -> Result<(), Error> {
        if let Some(existing) = self.find_mut(node, operation) {
            existing.status = OperationSnapshot { error: None, cleanup_error: None, id: operation, channel, state: OperationState::Active };
            existing.updated_ms = now_ms;
            return Ok(());
        }
        let slot = self
            .operations
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.operations
                    .iter()
                    .enumerate()
                    .filter_map(|(index, tracked)| {
                        tracked.filter(|tracked| tracked.status.state != OperationState::Active).map(|tracked| (index, tracked.updated_ms))
                    })
                    .min_by_key(|(_, updated_ms)| *updated_ms)
                    .map(|(index, _)| index)
            })
            .ok_or(Error::OperationRegistryFull)?;
        self.operations[slot] = Some(TrackedOperation {
            node,
            status: OperationSnapshot { error: None, cleanup_error: None, id: operation, channel, state: OperationState::Active },
            updated_ms: now_ms,
        });
        Ok(())
    }

    pub fn observe(&mut self, node: NodeId, status: OperationSnapshot, now_ms: u64) {
        if self.find_mut(node, status.id).is_none() {
            let _ = self.accepted(node, status.id, status.channel, now_ms);
        }
        if let Some(entry) = self.find_mut(node, status.id)
            && !entry.status.state.is_terminal()
        {
            entry.status = status;
            entry.updated_ms = now_ms;
        }
    }

    #[must_use]
    pub fn has_capacity_for(&self, needed: usize) -> bool {
        self.operations.iter().filter(|tracked| tracked.is_none_or(|tracked| tracked.status.state != OperationState::Active)).count()
            >= needed
    }

    pub fn terminal(&mut self, node: NodeId, operation: OperationId, channel: ChannelId, state: OperationState, now_ms: u64) {
        if let Some(tracked) = self.find_mut(node, operation) {
            if tracked.status.state == OperationState::Active {
                tracked.status.state = state;
                tracked.updated_ms = now_ms;
            }
        } else {
            // Freshly created as `Active` by `accepted()`, so the guard
            // above cannot have anything to reject yet: this unconditional
            // write is what actually lands the terminal state for an
            // operation the cache never tracked until this very fact.
            let _ = self.accepted(node, operation, channel, now_ms);
            if let Some(tracked) = self.find_mut(node, operation) {
                tracked.status.state = state;
            }
        }
    }

    #[must_use]
    pub fn status(&self, node: NodeId, operation: OperationId) -> Option<OperationSnapshot> {
        self.operations
            .iter()
            .flatten()
            .find(|tracked| tracked.node == node && tracked.status.id == operation)
            .map(|tracked| tracked.status)
    }

    #[must_use]
    pub fn has_active(&self, node: NodeId) -> bool {
        self.operations.iter().flatten().any(|tracked| tracked.node == node && tracked.status.state == OperationState::Active)
    }

    #[must_use]
    pub fn stale_active(&self, node: NodeId, now_ms: u64) -> Option<OperationId> {
        self.operations
            .iter()
            .flatten()
            .filter(|tracked| {
                tracked.node == node
                    && tracked.status.state == OperationState::Active
                    && now_ms.saturating_sub(tracked.updated_ms) >= RECONCILE_AFTER_MS
            })
            .min_by_key(|tracked| tracked.updated_ms)
            .map(|tracked| tracked.status.id)
    }

    pub fn clear_node(&mut self, node: NodeId) {
        for slot in &mut self.operations {
            if slot.is_some_and(|tracked| tracked.node == node) {
                *slot = None;
            }
        }
    }

    fn find_mut(&mut self, node: NodeId, operation: OperationId) -> Option<&mut TrackedOperation> {
        self.operations.iter_mut().flatten().find(|tracked| tracked.node == node && tracked.status.id == operation)
    }
}

impl<const OPERATIONS: usize> Default for OperationCache<OPERATIONS> {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduledRequest {
    #[must_use]
    pub const fn new(node: NodeId, request: Request, priority: Priority, ready_at_ms: u64, deadline_ms: u64) -> Self {
        Self { node, request, priority, ready_at_ms, deadline_ms, reply: None, order: 0 }
    }

    #[must_use]
    pub fn with_reply(mut self, reply: RequestId) -> Self {
        self.reply = Some(reply);
        self
    }
}

impl DiscoverySchedule {
    const fn new() -> Self {
        Self { due_ms: 0, settle_ms: 0 }
    }

    const fn interval_ms(&self, registered: usize, now_ms: u64) -> u64 {
        if registered == 0 || now_ms < self.settle_ms { DISCOVERY_SEARCH_INTERVAL_MS } else { DISCOVERY_SETTLED_INTERVAL_MS }
    }

    const fn is_due(&self, now_ms: u64) -> bool {
        now_ms >= self.due_ms
    }

    const fn settle_from(&mut self, now_ms: u64) {
        self.settle_ms = now_ms.saturating_add(DISCOVERY_SETTLE_WINDOW_MS);
    }

    const fn defer(&mut self, registered: usize, now_ms: u64) {
        self.due_ms = now_ms.saturating_add(self.interval_ms(registered, now_ms));
    }
}

impl<const NODES: usize, const QUEUED: usize> GatewayCore<NODES, QUEUED> {
    #[must_use]
    pub const fn new(maintenance_interval_ms: u64) -> Self {
        Self {
            nodes: [const { None }; NODES],
            requests: [const { None }; QUEUED],
            operations: OperationCache::new(),
            discovery: DiscoverySchedule::new(),
            maintenance_interval_ms,
            enqueue_order: 1,
            service_order: 1,
        }
    }

    #[must_use]
    pub const fn operations(&self) -> &OperationCache {
        &self.operations
    }

    pub const fn operations_mut(&mut self) -> &mut OperationCache {
        &mut self.operations
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.iter().flatten().count()
    }

    pub fn records(&self) -> impl Iterator<Item = &NodeRecord> {
        self.nodes.iter().flatten()
    }

    #[must_use]
    pub fn contains(&self, node: NodeId) -> bool {
        self.node_index(node).is_some()
    }

    #[must_use]
    pub fn nodes(&self, after: Option<NodeId>, gateway: &NodeSummary, now_ms: u64) -> NodePage {
        let mut nodes = heapless::Vec::<NodeSummary, NODE_PAGE_SIZE>::new();
        let mut cursor = after.map_or(0, |node| node.0);
        while !nodes.is_full() {
            let mut candidate_id = (gateway.info.id.0 > cursor).then_some(gateway.info.id.0);
            for info in self.nodes.iter().flatten().map(|record| &record.description.info) {
                if info.id.0 > cursor && candidate_id.is_none_or(|current| info.id.0 < current) {
                    candidate_id = Some(info.id.0);
                }
            }
            let Some(id) = candidate_id else {
                break;
            };
            cursor = id;
            let summary = if id == gateway.info.id.0 {
                gateway.clone()
            } else {
                self.nodes
                    .iter()
                    .flatten()
                    .find(|record| record.description.info.id.0 == id)
                    .map(|record| NodeSummary {
                        info: record.description.info.clone(),

                        silent_for_s: silence_seconds(now_ms, record.last_seen_ms),
                    })
                    .expect("candidate id was just selected from this same registry")
            };
            // The loop guard guarantees capacity for this push.
            let _ = nodes.push(summary);
        }
        let has_more = gateway.info.id.0 > cursor || self.nodes.iter().flatten().any(|record| record.description.info.id.0 > cursor);
        NodePage { nodes, next_after: has_more.then_some(NodeId(cursor)) }
    }

    #[must_use]
    pub fn node(&self, node: NodeId) -> Option<&NodeRecord> {
        self.node_index(node).and_then(|index| self.nodes[index].as_ref())
    }

    pub fn register(&mut self, description: NodeDescription, now_ms: u64) -> Result<(), Error> {
        if let Some(index) = self.node_index(description.info.id) {
            if let Some(record) = &mut self.nodes[index] {
                record.description = description;
                record.last_seen_ms = now_ms;
            }
            return Ok(());
        }

        let Some(index) = self.nodes.iter().position(Option::is_none) else {
            return Err(Error::RegistryFull);
        };
        let phase = maintenance_phase(description.info.id, self.maintenance_interval_ms);
        self.discovery.settle_from(now_ms);
        self.nodes[index] = Some(NodeRecord {
            description,
            session: SessionId::UNKNOWN,
            acknowledged_event: EventId(0),
            last_seen_ms: now_ms,
            maintenance_due_ms: now_ms.saturating_add(phase),
            consecutive_failures: 0,
            next_request_id: RequestId(1),
            last_served_order: 0,
        });
        Ok(())
    }

    pub fn absorb_discovery(
        &mut self, description: NodeDescription, session: SessionId, event: EventId, request_id: RequestId, now_ms: u64,
    ) -> Result<DiscoveryOutcome, Error> {
        let node = description.info.id;
        if self.contains(node) {
            self.register(description, now_ms)?;
            self.node_mut(node)?.consecutive_failures = 0;
            return Ok(DiscoveryOutcome::LivenessRefreshed);
        }
        self.register(description, now_ms)?;
        let _ = self.observe_cursor(node, session, event, now_ms);
        let _ = self.continue_after_request(node, request_id);
        let _ = self.enqueue(ScheduledRequest::new(
            node,
            Request::Node(wire_node::Request::Status(wire_node::Status)),
            Priority::Maintenance,
            now_ms,
            now_ms,
        ));
        Ok(DiscoveryOutcome::Enrolled)
    }

    pub fn allocate_request_id(&mut self, node: NodeId) -> Result<RequestId, Error> {
        let record = self.node_mut(node)?;
        let request_id = record.next_request_id;
        record.next_request_id.0 = record.next_request_id.0.wrapping_add(1).max(1);
        Ok(request_id)
    }

    pub fn continue_after_request(&mut self, node: NodeId, request_id: RequestId) -> Result<(), Error> {
        self.node_mut(node)?.next_request_id = RequestId(request_id.0.wrapping_add(1).max(1));
        Ok(())
    }

    pub fn acknowledgement(&self, node: NodeId) -> Result<(SessionId, EventId), Error> {
        let record = self.node(node).ok_or(Error::UnknownNode)?;
        Ok((record.session, record.acknowledged_event))
    }

    pub fn observe_response(&mut self, node: NodeId, session: SessionId, event: Option<Event>, now_ms: u64) -> Result<bool, Error> {
        self.observe_cursor(node, session, event.map_or(EventId(0), |event| event.id), now_ms)
    }

    pub fn observe_cursor(&mut self, node: NodeId, session: SessionId, event: EventId, now_ms: u64) -> Result<bool, Error> {
        let interval = self.maintenance_interval_ms;
        let record = self.node_mut(node)?;
        let session_changed = record.session != session;
        if session_changed {
            record.session = session;
            record.acknowledged_event = EventId(0);
        }
        if event.0 > record.acknowledged_event.0 {
            record.acknowledged_event = event;
        }
        record.last_seen_ms = now_ms;
        record.maintenance_due_ms = now_ms.saturating_add(interval);
        record.consecutive_failures = 0;
        Ok(session_changed)
    }

    pub fn observe_failure(&mut self, node: NodeId, now_ms: u64) -> Result<FailureOutcome, Error> {
        let interval = self.maintenance_interval_ms;
        let index = self.node_index(node).ok_or(Error::UnknownNode)?;
        let record = self.nodes[index].as_mut().unwrap();
        record.consecutive_failures = record.consecutive_failures.saturating_add(1);
        record.maintenance_due_ms = now_ms.saturating_add(interval);
        let silent_ms = now_ms.saturating_sub(record.last_seen_ms);
        if record.consecutive_failures < CONSECUTIVE_FAILURE_EVICTION_THRESHOLD || silent_ms < EVICTION_SILENCE_MS {
            return Ok(FailureOutcome::Retained);
        }
        self.nodes[index] = None;
        self.operations.clear_node(node);
        Ok(FailureOutcome::Evicted)
    }

    #[must_use]
    pub fn next_maintenance_deadline(&self) -> Option<u64> {
        self.nodes.iter().flatten().map(|record| record.maintenance_due_ms).min()
    }

    #[must_use]
    pub fn next_request_deadline(&self) -> Option<u64> {
        self.requests.iter().flatten().map(|request| request.ready_at_ms).min()
    }

    #[must_use]
    pub fn due_maintenance(&self, now_ms: u64) -> Option<NodeId> {
        self.nodes
            .iter()
            .flatten()
            .filter(|record| record.maintenance_due_ms <= now_ms)
            .min_by_key(|record| (record.maintenance_due_ms, record.last_served_order))
            .map(|record| record.description.info.id)
    }

    #[must_use]
    pub const fn due_discovery(&self, now_ms: u64) -> bool {
        self.discovery.is_due(now_ms)
    }

    #[must_use]
    pub const fn next_discovery_deadline(&self) -> u64 {
        self.discovery.due_ms
    }

    #[must_use]
    pub fn discovery_interval_ms(&self, now_ms: u64) -> u64 {
        self.discovery.interval_ms(self.node_count(), now_ms)
    }

    pub fn defer_discovery(&mut self, now_ms: u64) {
        let registered = self.node_count();
        self.discovery.defer(registered, now_ms);
    }

    pub fn bring_maintenance_forward(&mut self, node: NodeId, due_ms: u64) -> Result<(), Error> {
        let record = self.node_mut(node)?;
        record.maintenance_due_ms = record.maintenance_due_ms.min(due_ms);
        Ok(())
    }

    pub fn enqueue(&mut self, mut request: ScheduledRequest) -> Result<(), Error> {
        let occupied = self.requests.iter().flatten().count();
        let reserve = RESERVED_EMERGENCY_SLOTS.min(QUEUED);
        if request.priority != Priority::Emergency && occupied >= QUEUED.saturating_sub(reserve) {
            return Err(Error::QueueFull);
        }
        let Some(slot) = self.requests.iter_mut().find(|slot| slot.is_none()) else {
            return Err(Error::QueueFull);
        };
        request.order = self.enqueue_order;
        self.enqueue_order = self.enqueue_order.wrapping_add(1).max(1);
        *slot = Some(request);
        Ok(())
    }

    pub fn enqueue_checked(&mut self, request: ScheduledRequest) -> Result<(), Error> {
        let needed = motion_admission_capacity(&request.request);
        if needed > 0 && !self.operations.has_capacity_for(needed) {
            return Err(Error::OperationRegistryFull);
        }
        self.enqueue(request)
    }

    pub fn take_next(&mut self, now_ms: u64) -> Option<ScheduledRequest> {
        let mut selected: Option<usize> = None;
        for (index, request) in self.requests.iter().enumerate() {
            let Some(request) = request.as_ref().filter(|request| request.ready_at_ms <= now_ms) else {
                continue;
            };
            let better = match selected {
                None => true,
                Some(current) => self.precedes(request, self.requests[current].as_ref().expect("selected slot is occupied")),
            };
            if better {
                selected = Some(index);
            }
        }
        self.requests[selected?].take()
    }

    pub fn mark_served(&mut self, node: NodeId) -> Result<(), Error> {
        let order = self.service_order;
        self.service_order = self.service_order.wrapping_add(1).max(1);
        self.node_mut(node)?.last_served_order = order;
        Ok(())
    }

    fn precedes(&self, candidate: &ScheduledRequest, current: &ScheduledRequest) -> bool {
        let candidate_served = self.node(candidate.node).map_or(u32::MAX, |record| record.last_served_order);
        let current_served = self.node(current.node).map_or(u32::MAX, |record| record.last_served_order);
        (candidate.priority, candidate.deadline_ms, candidate_served, candidate.order)
            < (current.priority, current.deadline_ms, current_served, current.order)
    }

    fn node_index(&self, node: NodeId) -> Option<usize> {
        self.nodes.iter().position(|record| record.as_ref().is_some_and(|record| record.description.info.id == node))
    }

    fn node_mut(&mut self, node: NodeId) -> Result<&mut NodeRecord, Error> {
        let index = self.node_index(node).ok_or(Error::UnknownNode)?;
        Ok(self.nodes[index].as_mut().unwrap())
    }
}

/* === Public Functions === */

/* === Private Functions === */

const fn motion_admission_capacity(request: &Request) -> usize {
    match request {
        Request::Motion(motion::Request::MoveTogether(_)) => 2,
        Request::Motion(
            motion::Request::MoveTo(_) | motion::Request::MoveRelative(_) | motion::Request::SetHome(_) | motion::Request::Stop(_),
        ) => 1,
        _ => 0,
    }
}

fn maintenance_phase(node: NodeId, interval_ms: u64) -> u64 {
    if interval_ms == 0 {
        return 0;
    }
    u64::from(node.0).wrapping_mul(2_654_435_761).wrapping_rem(interval_ms)
}

fn silence_seconds(now_ms: u64, last_seen_ms: u64) -> u16 {
    u16::try_from(now_ms.saturating_sub(last_seen_ms) / 1_000).unwrap_or(u16::MAX)
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::{
        motion,
        node::{self, FIRMWARE_VERSION, NodeInfo},
    };

    use super::*;

    fn info(id: u32) -> NodeInfo {
        NodeInfo { node_type: hnm_protocol::nodes::NodeType::Sidecar, id: NodeId(id), firmware: FIRMWARE_VERSION }
    }

    fn description(id: u32) -> NodeDescription {
        NodeDescription { info: info(id), satellites: heapless::Vec::new() }
    }

    fn poll(node: NodeId, priority: Priority, deadline_ms: u64) -> ScheduledRequest {
        ScheduledRequest::new(node, Request::Node(node::Request::Poll(node::Poll)), priority, 0, deadline_ms)
    }

    #[test]
    fn operation_cache_events() {
        let node = NodeId(566_114);
        let channel = ChannelId(1);
        let operation = OperationId(42);
        let mut cache = OperationCache::<2>::new();
        cache.accepted(node, operation, channel, 10).unwrap();
        assert!(cache.has_active(node));
        assert_eq!(
            cache.status(node, operation),
            Some(OperationSnapshot { error: None, cleanup_error: None, id: operation, channel, state: OperationState::Active })
        );
        cache.terminal(node, operation, channel, OperationState::Completed, 20);
        assert!(!cache.has_active(node));
        assert_eq!(cache.status(node, operation).unwrap().state, OperationState::Completed);
    }

    #[test]
    fn terminal_operations_operations() {
        let node = NodeId(566_114);
        let mut cache = OperationCache::<2>::new();
        cache.accepted(node, OperationId(1), ChannelId(1), 1).unwrap();
        cache.accepted(node, OperationId(2), ChannelId(2), 2).unwrap();
        cache.terminal(node, OperationId(1), ChannelId(1), OperationState::Completed, 3);
        cache.accepted(node, OperationId(3), ChannelId(1), 4).unwrap();
        assert!(cache.status(node, OperationId(1)).is_none());
        assert!(cache.status(node, OperationId(2)).is_some());
        assert!(cache.status(node, OperationId(3)).is_some());
    }

    #[test]
    fn capacity_pressure_active() {
        let node = NodeId(566_114);
        let mut cache = OperationCache::<MAX_TRACKED_OPERATIONS>::new();

        // The two channels a real sweep drives concurrently: accepted first
        // and never completed for the rest of this test.
        let active_a = OperationId(1);
        let active_b = OperationId(2);
        cache.accepted(node, active_a, ChannelId(1), 0).unwrap();
        cache.accepted(node, active_b, ChannelId(2), 0).unwrap();

        // Fill every remaining slot with terminal churn from "earlier
        // cycles", each stamped with a distinct `updated_ms` so eviction
        // order is meaningful.
        for id in 3..=u32::try_from(MAX_TRACKED_OPERATIONS).unwrap() {
            let operation = OperationId(id);
            cache.accepted(node, operation, ChannelId(1), u64::from(id)).unwrap();
            cache.terminal(node, operation, ChannelId(1), OperationState::Completed, u64::from(id));
        }

        // The cache is now completely full. Accepting far more than
        // capacity again must keep succeeding by evicting terminal
        // entries -- never the two still-Active ones, and never failing
        // with the registry full.
        let next = u32::try_from(MAX_TRACKED_OPERATIONS).unwrap() + 1;
        for id in next..next + 200 {
            let operation = OperationId(id);
            cache.accepted(node, operation, ChannelId(1), u64::from(id)).unwrap();
            cache.terminal(node, operation, ChannelId(1), OperationState::Completed, u64::from(id));

            assert_eq!(cache.status(node, active_a).unwrap().state, OperationState::Active, "operation {id} evicted an active entry");
            assert_eq!(cache.status(node, active_b).unwrap().state, OperationState::Active, "operation {id} evicted an active entry");
        }
        assert!(cache.has_active(node));
    }

    #[test]
    fn movetogether_room_it() {
        let node = NodeId(566_114);
        let mut coordinator: GatewayCore = GatewayCore::new(30_000);
        coordinator.register(description(node.0), 0).unwrap();

        // Saturate every slot but one with still-active entries, leaving
        // exactly one free -- enough for a single-operation admission, not
        // enough for MoveTogether's two.
        for id in 1..u32::try_from(MAX_TRACKED_OPERATIONS).unwrap() {
            coordinator.operations_mut().accepted(node, OperationId(id), ChannelId(1), 0).unwrap();
        }
        assert!(coordinator.operations().has_capacity_for(1));
        assert!(!coordinator.operations().has_capacity_for(2));

        let move_together = ScheduledRequest::new(
            node,
            Request::Motion(motion::Request::MoveTogether(motion::MoveTogether { moves: heapless::Vec::new() })),
            Priority::Interactive,
            0,
            100,
        );
        assert_eq!(
            coordinator.enqueue_checked(move_together),
            Err(Error::OperationRegistryFull),
            "a MoveTogether needing two slots must be refused with only one free"
        );

        // The node never sees this exchange at all: nothing was queued.
        assert!(coordinator.take_next(0).is_none(), "a refused admission must never reach the scheduling queue");

        // A single-operation admission still fits in the one free slot.
        let move_to = ScheduledRequest::new(
            node,
            Request::Motion(motion::Request::MoveTo(motion::MoveTo {
                channel: ChannelId(2),
                target: motion::Target::Rotational { microdegrees: 0 },
                settings: motion::Settings::default(),
            })),
            Priority::Interactive,
            0,
            100,
        );
        assert!(coordinator.enqueue_checked(move_to).is_ok());
        assert!(coordinator.take_next(0).is_some());
    }

    #[test]
    fn registry_holds_maximum() {
        let mut coordinator = GatewayCore::<MAX_MANAGED_NODES, 4>::new(10_000);
        for offset in 0..MAX_MANAGED_NODES {
            coordinator.register(description(100_000 + u32::try_from(offset).unwrap()), 0).unwrap();
        }
        assert_eq!(coordinator.node_count(), MAX_MANAGED_NODES);
        assert_eq!(coordinator.register(description(999_999), 0), Err(Error::RegistryFull));
    }

    #[test]
    fn node_pages_registry() {
        let mut core = GatewayCore::<4, 1>::new(10_000);
        core.register(description(400_004), 0).unwrap();
        core.register(description(200_002), 0).unwrap();
        core.register(NodeDescription { ..description(300_003) }, 0).unwrap();
        core.register(description(500_005), 0).unwrap();
        // v7: the gateway's own roster entry carries capabilities resolved
        // by the caller (`mesh::coordinator::node_summary`), not queried
        // here -- `nodes()` never touches the network either way.
        let gateway = NodeSummary { info: info(100_001), silent_for_s: 0 };

        let first = core.nodes(None, &gateway, 0);
        let ids: heapless::Vec<NodeId, NODE_PAGE_SIZE> = first.nodes.iter().map(|node| node.info.id).collect();
        assert_eq!(ids.as_slice(), &[NodeId(100_001), NodeId(200_002), NodeId(300_003), NodeId(400_004),]);

        assert_eq!(first.next_after, Some(NodeId(400_004)));

        let second = core.nodes(first.next_after, &gateway, 0);
        assert_eq!(second.nodes[0].info.id, NodeId(500_005));
        assert_eq!(second.next_after, None);
    }

    #[test]
    fn node_pages_time() {
        let mut core = GatewayCore::<2, 1>::new(10_000);
        core.register(description(200_002), 1_000).unwrap();
        let gateway = NodeSummary {
            info: info(100_001),

            // Never actually silent from itself; asserted unchanged below to
            // prove `nodes()` never recomputes the gateway's own entry.
            silent_for_s: 0,
        };

        let page = core.nodes(None, &gateway, 13_000);
        assert_eq!(page.nodes[0].info.id, NodeId(100_001));
        assert_eq!(page.nodes[0].silent_for_s, 0, "the gateway's own entry passes through whatever the caller supplied");
        assert_eq!(page.nodes[1].info.id, NodeId(200_002));
        assert_eq!(page.nodes[1].silent_for_s, 12, "silence is (now_ms - last_seen_ms) in whole seconds, resolved at response time");
    }

    #[test]
    fn sequential_ids_deadlines() {
        let interval_ms = 10_000;
        let mut coordinator = GatewayCore::<MAX_MANAGED_NODES, 4>::new(interval_ms);
        let mut deadlines = [0_u64; MAX_MANAGED_NODES];
        for (offset, deadline) in deadlines.iter_mut().enumerate() {
            let node = 100_000 + u32::try_from(offset).unwrap();
            coordinator.register(description(node), 0).unwrap();
            *deadline = coordinator.node(NodeId(node)).unwrap().maintenance_due_ms;
        }
        deadlines.sort_unstable();
        assert!(deadlines.windows(2).all(|pair| pair[0] != pair[1]));
        // A flat millisecond bound only made sense against the discarded
        // 100-node ceiling's much denser sequential-id spread; expressed
        // relationally instead, no adjacent pair may sit further apart than
        // four times the uniform average spacing for however many nodes
        // `MAX_MANAGED_NODES` actually allows.
        let average_gap_ms = interval_ms / u64::try_from(MAX_MANAGED_NODES).unwrap();
        assert!(deadlines.windows(2).all(|pair| pair[1].saturating_sub(pair[0]) <= 4 * average_gap_ms));
    }

    #[test]
    fn new_boot_cursor() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 2>::new(10_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator
            .observe_response(
                node,
                SessionId(10),
                Some(Event {
                    id: EventId(9),
                    payload: hnm_protocol::wire::EventPayload::Motion(motion::Event::Stopped {
                        channel: ChannelId(1),
                        operation: OperationId(1),
                    }),
                }),
                5,
            )
            .unwrap();
        assert_eq!(coordinator.acknowledgement(node).unwrap().1, EventId(9));

        assert!(coordinator.observe_response(node, SessionId(11), None, 10).unwrap());
        assert_eq!(coordinator.acknowledgement(node).unwrap(), (SessionId(11), EventId(0)));
    }

    #[test]
    fn absorbing_new_poll() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(10_000);
        assert_eq!(
            coordinator.absorb_discovery(description(node.0), SessionId(7), EventId(0), RequestId(1), 5,).unwrap(),
            DiscoveryOutcome::Enrolled
        );

        assert!(coordinator.contains(node));
        assert_eq!(coordinator.acknowledgement(node).unwrap(), (SessionId(7), EventId(0)));
        let scheduled = coordinator.take_next(5).unwrap();
        assert_eq!(scheduled.node, node);
        assert_eq!(scheduled.priority, Priority::Maintenance);
        assert_eq!(scheduled.request, Request::Node(node::Request::Status(node::Status)));
    }

    #[test]
    fn absorbing_node_poll() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(10_000);
        coordinator.absorb_discovery(description(node.0), SessionId(7), EventId(0), RequestId(1), 5).unwrap();
        // A second response heard for the same node inside the same
        // collection window is a liveness refresh, not a new enrollment.
        assert_eq!(
            coordinator.absorb_discovery(description(node.0), SessionId(7), EventId(0), RequestId(1), 6,).unwrap(),
            DiscoveryOutcome::LivenessRefreshed
        );

        coordinator.take_next(6).unwrap();
        assert!(coordinator.take_next(6).is_none());
    }

    #[test]
    fn broadcast_reply_state() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.observe_cursor(node, SessionId(42), EventId(7), 0).unwrap();
        coordinator.operations_mut().accepted(node, OperationId(1), ChannelId(1), 0).unwrap();
        // A prior failed exchange, still below the eviction threshold: a
        // heard broadcast reply is proof of life and should clear it.
        coordinator.observe_failure(node, 1).unwrap();

        // Absorbing a broadcast reply from this already-known node, exactly
        // as `mesh::coordinator::absorb_broadcast_response` does for every
        // response heard during one collection window.
        let outcome = coordinator.absorb_discovery(description(node.0), SessionId(42), EventId(0), RequestId(9), 2_000).unwrap();

        assert_eq!(outcome, DiscoveryOutcome::LivenessRefreshed);
        assert_eq!(
            coordinator.acknowledgement(node).unwrap(),
            (SessionId(42), EventId(7)),
            "the event cursor must be untouched by a broadcast echo"
        );
        assert!(
            coordinator.operations().status(node, OperationId(1)).is_some(),
            "a broadcast echo from an already-registered node must not clear its tracked operations"
        );
        assert!(coordinator.operations().has_active(node));
        assert_eq!(coordinator.node(node).unwrap().last_seen_ms, 2_000);
        assert_eq!(coordinator.node(node).unwrap().consecutive_failures, 0);
    }

    #[test]
    fn enrollment_request_command() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<1, 1>::new(10_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.continue_after_request(node, RequestId(41)).unwrap();
        assert_eq!(coordinator.allocate_request_id(node), Ok(RequestId(42)));
    }

    #[test]
    fn request_ids_zero() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<1, 1>::new(10_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.node_mut(node).unwrap().next_request_id = RequestId(u32::MAX);

        assert_eq!(coordinator.allocate_request_id(node), Ok(RequestId(u32::MAX)));
        assert_eq!(coordinator.allocate_request_id(node), Ok(RequestId(1)));
    }

    #[test]
    fn emergency_capacity_reserved() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<1, 4>::new(10_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.enqueue(poll(node, Priority::Maintenance, 10)).unwrap();
        coordinator.enqueue(poll(node, Priority::Maintenance, 20)).unwrap();
        assert_eq!(coordinator.enqueue(poll(node, Priority::Maintenance, 30)), Err(Error::QueueFull));
        coordinator.enqueue(poll(node, Priority::Emergency, 30)).unwrap();
        coordinator.enqueue(poll(node, Priority::Emergency, 40)).unwrap();
    }

    #[test]
    fn priority_precedes_deadline() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<1, 4>::new(10_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.enqueue(poll(node, Priority::Maintenance, 1)).unwrap();
        coordinator.enqueue(poll(node, Priority::Interactive, 100)).unwrap();
        assert_eq!(coordinator.take_next(0).unwrap().priority, Priority::Interactive);
    }

    #[test]
    fn selected_job_token() {
        let node = NodeId(100_001);
        let mut core = GatewayCore::<1, 4>::new(10_000);
        core.register(description(node.0), 0).unwrap();
        core.enqueue(poll(node, Priority::Interactive, 10).with_reply(RequestId(42))).unwrap();

        assert_eq!(core.take_next(0).unwrap().reply, Some(RequestId(42)));
    }

    #[test]
    fn discovery_searches_empty() {
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        assert!(coordinator.due_discovery(0));

        coordinator.defer_discovery(0);
        assert!(!coordinator.due_discovery(DISCOVERY_SEARCH_INTERVAL_MS - 1));
        assert!(coordinator.due_discovery(DISCOVERY_SEARCH_INTERVAL_MS));
        assert_eq!(coordinator.next_discovery_deadline(), DISCOVERY_SEARCH_INTERVAL_MS);
    }

    #[test]
    fn discovery_backs_settled() {
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.register(description(100_001), 0).unwrap();

        // The settling window keeps searching so a batch powered together
        // enrolls without waiting a settled interval each.
        assert_eq!(coordinator.discovery_interval_ms(DISCOVERY_SETTLE_WINDOW_MS - 1), DISCOVERY_SEARCH_INTERVAL_MS);

        let settled = DISCOVERY_SETTLE_WINDOW_MS;
        assert_eq!(coordinator.discovery_interval_ms(settled), DISCOVERY_SETTLED_INTERVAL_MS);
        coordinator.defer_discovery(settled);
        assert!(!coordinator.due_discovery(settled + DISCOVERY_SETTLED_INTERVAL_MS - 1));
        assert!(coordinator.due_discovery(settled + DISCOVERY_SETTLED_INTERVAL_MS));
        const { assert!(DISCOVERY_SETTLED_INTERVAL_MS >= DISCOVERY_SEARCH_INTERVAL_MS * 10) }
    }

    #[test]
    fn newly_enrolled_window() {
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.register(description(100_001), 0).unwrap();
        let settled = DISCOVERY_SETTLE_WINDOW_MS;
        assert_eq!(coordinator.discovery_interval_ms(settled), DISCOVERY_SETTLED_INTERVAL_MS);

        coordinator.register(description(100_002), settled).unwrap();
        assert_eq!(coordinator.discovery_interval_ms(settled), DISCOVERY_SEARCH_INTERVAL_MS);

        // A refreshed description is not a new enrollment.
        let reopened = settled + DISCOVERY_SETTLE_WINDOW_MS;
        coordinator.register(description(100_002), reopened).unwrap();
        assert_eq!(coordinator.discovery_interval_ms(reopened), DISCOVERY_SETTLED_INTERVAL_MS);
    }

    #[test]
    fn discovery_recovers_empties() {
        let mut schedule = DiscoverySchedule::new();
        schedule.settle_from(0);
        let settled = DISCOVERY_SETTLE_WINDOW_MS;
        assert_eq!(schedule.interval_ms(1, settled), DISCOVERY_SETTLED_INTERVAL_MS);
        assert_eq!(schedule.interval_ms(0, settled), DISCOVERY_SEARCH_INTERVAL_MS);

        schedule.defer(0, settled);
        assert!(schedule.is_due(settled + DISCOVERY_SEARCH_INTERVAL_MS));
    }

    #[test]
    fn discovery_never_work() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<1, 8>::new(30_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.enqueue(poll(node, Priority::Discovery, 1)).unwrap();
        coordinator.enqueue(poll(node, Priority::Maintenance, 50)).unwrap();
        coordinator.enqueue(poll(node, Priority::Interactive, 100)).unwrap();

        assert_eq!(coordinator.take_next(0).unwrap().priority, Priority::Interactive);
        assert_eq!(coordinator.take_next(0).unwrap().priority, Priority::Maintenance);
        assert_eq!(coordinator.take_next(0).unwrap().priority, Priority::Discovery);
    }

    #[test]
    fn equal_priority_node() {
        let first = NodeId(100_001);
        let second = NodeId(100_002);
        let mut coordinator = GatewayCore::<2, 4>::new(10_000);
        coordinator.register(description(first.0), 0).unwrap();
        coordinator.register(description(second.0), 0).unwrap();
        coordinator.mark_served(first).unwrap();
        coordinator.enqueue(poll(first, Priority::Maintenance, 10)).unwrap();
        coordinator.enqueue(poll(second, Priority::Maintenance, 10)).unwrap();
        assert_eq!(coordinator.take_next(0).unwrap().node, second);
    }

    #[test]
    fn rapid_fire_evict() {
        // Simulates the real fast path this eviction bound must survive: an
        // active tracked operation is re-polled roughly every 250ms under
        // the INTERACTIVE policy's ~680ms failed-exchange budget, so the
        // failure count alone would cross the threshold within seconds.
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.operations_mut().accepted(node, OperationId(1), ChannelId(1), 0).unwrap();

        let mut now_ms = 0;
        for _ in 0..50 {
            now_ms += 680;
            assert_eq!(coordinator.observe_failure(node, now_ms).unwrap(), FailureOutcome::Retained);
        }

        // 50 * 680ms is ~34s, far past the failure-count threshold but far
        // short of EVICTION_SILENCE_MS: the node stays registered, so a
        // caller still gets Unavailable (retryable) rather than a NotFound
        // it can never recover from mid-operation.
        assert!(now_ms < EVICTION_SILENCE_MS);
        assert!(coordinator.contains(node));
        assert!(coordinator.operations().has_active(node));
    }

    #[test]
    fn eviction_waits_crosses() {
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.register(description(node.0), 0).unwrap();

        // The failure count crosses the threshold almost immediately...
        let mut now_ms = 0;
        for _ in 0..CONSECUTIVE_FAILURE_EVICTION_THRESHOLD {
            now_ms += 100;
            assert_eq!(coordinator.observe_failure(node, now_ms).unwrap(), FailureOutcome::Retained);
        }
        assert!(coordinator.contains(node), "the failure count alone must never evict");

        // ...but eviction still waits for EVICTION_SILENCE_MS of wall-clock
        // silence measured from the node's last real response.
        assert_eq!(coordinator.observe_failure(node, EVICTION_SILENCE_MS - 1).unwrap(), FailureOutcome::Retained);
        assert!(coordinator.contains(node));

        assert_eq!(coordinator.observe_failure(node, EVICTION_SILENCE_MS).unwrap(), FailureOutcome::Evicted);
        assert!(!coordinator.contains(node));
    }

    #[test]
    fn node_that_rediscovered() {
        // Mirrors the gateway's real MAINTENANCE_INTERVAL_MS cadence
        // (coordinator/mod.rs): at that cadence the failure count and the
        // EVICTION_SILENCE_MS wall-clock gate cross together, eight failed
        // maintenance cycles apart.
        const MAINTENANCE_INTERVAL_MS: u64 = 30_000;
        let node = NodeId(100_001);
        let mut coordinator = GatewayCore::<2, 4>::new(MAINTENANCE_INTERVAL_MS);
        coordinator.register(description(node.0), 0).unwrap();
        coordinator.operations_mut().accepted(node, OperationId(1), ChannelId(1), 0).unwrap();

        let mut now_ms = 0;
        for _ in 0..CONSECUTIVE_FAILURE_EVICTION_THRESHOLD - 1 {
            now_ms += MAINTENANCE_INTERVAL_MS;
            assert_eq!(coordinator.observe_failure(node, now_ms).unwrap(), FailureOutcome::Retained);
            assert!(coordinator.contains(node));
        }

        now_ms += MAINTENANCE_INTERVAL_MS;
        assert_eq!(coordinator.observe_failure(node, now_ms).unwrap(), FailureOutcome::Evicted);
        assert!(!coordinator.contains(node));
        assert_eq!(coordinator.node_count(), 0);
        assert!(coordinator.operations().status(node, OperationId(1)).is_none());
        assert!(!coordinator.operations().has_active(node));

        // The freed identity is unregistered, not merely unreachable.
        assert_eq!(coordinator.observe_failure(node, now_ms), Err(Error::UnknownNode));

        // When the node answers a sweep again, it enrolls as genuinely new
        // and the discovery search window reopens for it.
        coordinator.register(description(node.0), now_ms).unwrap();
        assert!(coordinator.contains(node));
        assert_eq!(coordinator.discovery_interval_ms(now_ms), DISCOVERY_SEARCH_INTERVAL_MS);
    }

    #[test]
    fn freshly_accepted_reconciliation() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.operations_mut().accepted(node, operation, ChannelId(1), 0).unwrap();

        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS - 1), None);
    }

    #[test]
    fn operation_active_due() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.operations_mut().accepted(node, operation, ChannelId(1), 0).unwrap();

        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS), Some(operation));
    }

    #[test]
    fn reconciling_operation_clock() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let channel = ChannelId(1);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.operations_mut().accepted(node, operation, channel, 0).unwrap();
        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS), Some(operation));

        // A reconciliation attempt -- successful or not -- that confirms
        // the operation is still Active touches `updated_ms`, so it does
        // not fire again on every single tick while genuinely still moving.
        coordinator.operations_mut().accepted(node, operation, channel, RECONCILE_AFTER_MS).unwrap();
        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS + 1), None);
        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS + RECONCILE_AFTER_MS), Some(operation));
    }

    #[test]
    fn terminal_operation_never() {
        let node = NodeId(566_114);
        let operation = OperationId(1);
        let channel = ChannelId(1);
        let mut coordinator = GatewayCore::<2, 4>::new(30_000);
        coordinator.operations_mut().accepted(node, operation, channel, 0).unwrap();
        coordinator.operations_mut().terminal(node, operation, channel, OperationState::Completed, 0);

        assert_eq!(coordinator.operations().stale_active(node, RECONCILE_AFTER_MS * 10), None);
    }
}
