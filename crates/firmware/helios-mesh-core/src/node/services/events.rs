/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Chassis-owned pending capability events with cumulative-ack compaction.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh_protocol::{
    node::{EventId, NodeId, SessionId},
    wire::{Event, EventPayload},
};

/* === Constants === */

pub const EVENT_QUEUE_CAPACITY: usize = 8;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pending {
    peer: NodeId,
    event: Event,
}

pub struct Events<const N: usize = EVENT_QUEUE_CAPACITY> {
    pending: [Option<Pending>; N],
    next_id: u32,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<const N: usize> Events<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self { pending: [None; N], next_id: 1 }
    }

    pub fn queue(&mut self, peer: NodeId, payload: EventPayload) -> Option<Event> {
        if N > 0 && self.pending.iter().all(Option::is_some) {
            for index in 0..N - 1 {
                self.pending[index] = self.pending[index + 1];
            }
            self.pending[N - 1] = None;
        }
        let slot = self.pending.iter_mut().find(|slot| slot.is_none())?;
        let event = Event { id: EventId(self.next_id), payload };
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(Pending { peer, event });
        Some(event)
    }

    pub fn next_after(
        &mut self, peer: NodeId, session: SessionId, acknowledged_session: SessionId, acknowledged_event: EventId,
    ) -> Option<Event> {
        let mut cursor = 0;
        while cursor < N {
            let acknowledged = session == acknowledged_session
                && self.pending[cursor].is_some_and(|pending| pending.peer == peer && pending.event.id.0 <= acknowledged_event.0);
            if acknowledged {
                for index in cursor..N - 1 {
                    self.pending[index] = self.pending[index + 1];
                }
                self.pending[N - 1] = None;
            } else {
                cursor += 1;
            }
        }
        self.pending.iter().flatten().find(|pending| pending.peer == peer).map(|pending| pending.event)
    }
}

impl<const N: usize> Default for Events<N> {
    fn default() -> Self {
        Self::new()
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use helios_mesh_protocol::{
        motion,
        node::{ChannelId, OperationId},
    };

    use super::*;

    const PEER: NodeId = NodeId(273_282);
    const OTHER: NodeId = NodeId(100_001);
    const SESSION: SessionId = SessionId(9);

    fn payload(operation: u32) -> EventPayload {
        EventPayload::Motion(motion::Event::Stopped { channel: ChannelId(1), operation: OperationId(operation) })
    }

    #[test]
    fn cumulative_acknowledgment_events() {
        let mut events = Events::<4>::new();
        let first = events.queue(PEER, payload(1)).unwrap();
        let second = events.queue(PEER, payload(2)).unwrap();
        let third = events.queue(PEER, payload(3)).unwrap();
        assert_eq!(first.id, EventId(1));

        let oldest = events.next_after(PEER, SESSION, SESSION, second.id);
        assert_eq!(oldest, Some(third));

        // Acknowledging the last event empties the peer's queue.
        assert_eq!(events.next_after(PEER, SESSION, SESSION, third.id), None);
    }

    #[test]
    fn acknowledgments_stale_nothing() {
        let mut events = Events::<4>::new();
        let first = events.queue(PEER, payload(1)).unwrap();

        let oldest = events.next_after(PEER, SESSION, SessionId(8), first.id);
        assert_eq!(oldest, Some(first));
    }

    #[test]
    fn events_keyed_peer() {
        let mut events = Events::<4>::new();
        let ours = events.queue(PEER, payload(1)).unwrap();
        let theirs = events.queue(OTHER, payload(2)).unwrap();

        // Acknowledging one peer's event leaves the other queue untouched.
        assert_eq!(events.next_after(PEER, SESSION, SESSION, ours.id), None);
        assert_eq!(events.next_after(OTHER, SESSION, SESSION, EventId(0)), Some(theirs));
    }

    #[test]
    fn full_queue_event() {
        let mut events = Events::<2>::new();
        events.queue(PEER, payload(1)).unwrap();
        let second = events.queue(PEER, payload(2)).unwrap();
        let third = events.queue(PEER, payload(3)).unwrap();

        // The oldest event (1) was evicted to admit the newest; the two that
        // remain are still delivered, in order.
        assert_eq!(events.next_after(PEER, SESSION, SESSION, EventId(0)), Some(second));
        assert_eq!(events.next_after(PEER, SESSION, SESSION, second.id), Some(third));
    }

    #[test]
    fn terminal_event_survives() {
        let mut events = Events::<2>::new();
        events.queue(PEER, payload(1)).unwrap();
        let second = events.queue(PEER, payload(2)).unwrap();
        let terminal =
            events.queue(PEER, EventPayload::Motion(motion::Event::Failed { channel: ChannelId(1), operation: OperationId(3) })).unwrap();

        // A terminal completion queued onto an already-full queue is not the
        // one dropped, and is still delivered once the peer catches up.
        assert_eq!(events.next_after(PEER, SESSION, SESSION, second.id), Some(terminal));
    }

    #[test]
    fn burst_that_event() {
        let mut events = Events::<4>::new();
        let target = events.queue(PEER, payload(1)).unwrap();
        // Three more events land before anything ever reads the queue --
        // e.g. a second channel's own completions, or repeated facts from
        // retried admissions -- overflowing capacity and evicting `target`
        // before it was ever delivered.
        for operation in 2..=5 {
            events.queue(PEER, payload(operation)).unwrap();
        }

        // Draining everything the queue will ever yield never produces the
        // event that was queued first.
        let mut delivered = heapless::Vec::<Event, 8>::new();
        let mut acknowledged = EventId(0);
        while let Some(event) = events.next_after(PEER, SESSION, SESSION, acknowledged) {
            acknowledged = event.id;
            delivered.push(event).unwrap();
        }
        assert!(!delivered.contains(&target), "the early event must have been evicted, not merely delayed");
    }
}
