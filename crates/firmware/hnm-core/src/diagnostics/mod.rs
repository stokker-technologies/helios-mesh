/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Dispatcher-owned ephemeral diagnostics shared with node residents.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::cell::RefCell;

// Third Party Imports
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::Instant;

// Local Imports
use crate::journal::Journal;
use hnm_protocol::{
    diagnostics::{JournalEvent, JournalPage, NodeDiagnostics, RequestDiagnostics},
    node::NodeId,
};

/* === Constants === */

const JOURNAL_CAPACITY: usize = 64;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Diagnostics {
    state: Mutex<CriticalSectionRawMutex, RefCell<State>>,
}

struct State {
    journal: Journal<JOURNAL_CAPACITY>,
    requests: RequestDiagnostics,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Diagnostics {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(RefCell::new(State {
                journal: Journal::new(),
                requests: RequestDiagnostics { received: 0, succeeded: 0, failed: 0 },
            })),
        }
    }

    pub fn record(&self, event: JournalEvent) {
        self.state.lock(|state| {
            state.borrow_mut().record(Instant::now().as_millis(), event);
        });
    }

    pub fn request_received(&self) {
        self.state.lock(|state| {
            state.borrow_mut().request_received();
        });
    }

    pub fn request_completed(&self, succeeded: bool) {
        self.state.lock(|state| {
            state.borrow_mut().request_completed(succeeded);
        });
    }

    #[must_use]
    pub fn page(&self, after_sequence: u32) -> JournalPage {
        self.state.lock(|state| state.borrow().journal.page(after_sequence))
    }

    #[must_use]
    pub fn snapshot(&self, node: NodeId) -> NodeDiagnostics {
        self.state.lock(|state| state.borrow().snapshot(node, Instant::now().as_millis()))
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    fn record(&mut self, timestamp_ms: u64, event: JournalEvent) {
        self.journal.push(timestamp_ms, event);
    }

    fn request_received(&mut self) {
        self.requests.received = self.requests.received.saturating_add(1);
    }

    fn request_completed(&mut self, succeeded: bool) {
        if succeeded {
            self.requests.succeeded = self.requests.succeeded.saturating_add(1);
        } else {
            self.requests.failed = self.requests.failed.saturating_add(1);
        }
    }

    fn snapshot(&self, node: NodeId, uptime_ms: u64) -> NodeDiagnostics {
        NodeDiagnostics { node, uptime_ms, requests: self.requests, journal: self.journal.summary() }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::{
        diagnostics::JournalEvent,
        node::{NodeId, RequestId},
    };

    use super::{JOURNAL_CAPACITY, State};
    use crate::journal::Journal;

    #[test]
    fn snapshot_projects_summary() {
        let mut state = State {
            journal: Journal::<JOURNAL_CAPACITY>::new(),
            requests: hnm_protocol::diagnostics::RequestDiagnostics { received: 0, succeeded: 0, failed: 0 },
        };
        state.request_received();
        state.request_completed(true);
        state.request_received();
        state.request_completed(false);
        state.record(42, JournalEvent::MeshRequestReceived { peer: NodeId(273_282), request_id: RequestId(7) });

        let snapshot = state.snapshot(NodeId(566_114), 84);

        assert_eq!(snapshot.node, NodeId(566_114));
        assert_eq!(snapshot.uptime_ms, 84);
        assert_eq!(snapshot.requests.received, 2);
        assert_eq!(snapshot.requests.succeeded, 1);
        assert_eq!(snapshot.requests.failed, 1);
        assert_eq!(snapshot.journal.retained, 1);
        assert_eq!(snapshot.journal.oldest_sequence, 1);
        assert_eq!(snapshot.journal.latest_sequence, 1);
        assert_eq!(snapshot.journal.overwritten, 0);
    }
}
