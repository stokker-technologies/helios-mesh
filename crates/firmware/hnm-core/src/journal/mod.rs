/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Fixed-capacity in-memory diagnostic journal.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm_protocol::diagnostics::{JournalEntry, JournalEvent, JournalPage, JournalSummary};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy)]
pub struct Journal<const N: usize> {
    entries: [Option<JournalEntry>; N],
    next_slot: usize,
    next_sequence: u32,
    overwritten: u32,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<const N: usize> Journal<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: [None; N], next_slot: 0, next_sequence: 1, overwritten: 0 }
    }

    pub fn push(&mut self, timestamp_ms: u64, event: JournalEvent) {
        if N == 0 {
            return;
        }
        if self.entries[self.next_slot].is_some() {
            self.overwritten = self.overwritten.saturating_add(1);
        }
        self.entries[self.next_slot] = Some(JournalEntry { sequence: self.next_sequence, timestamp_ms, event });
        self.next_slot = (self.next_slot + 1) % N;
        self.next_sequence = self.next_sequence.wrapping_add(1).max(1);
    }

    pub fn contains(&self, predicate: impl Fn(JournalEntry) -> bool) -> bool {
        self.entries.iter().flatten().copied().any(predicate)
    }

    #[must_use]
    pub fn page_where(&self, after_sequence: u32, predicate: impl Fn(JournalEntry) -> bool) -> JournalPage {
        let mut entries = heapless::Vec::new();
        let mut cursor = after_sequence;
        while !entries.is_full() {
            let next = self
                .entries
                .iter()
                .flatten()
                .copied()
                .filter(|entry| entry.sequence > cursor && predicate(*entry))
                .min_by_key(|entry| entry.sequence);
            let Some(next) = next else {
                break;
            };
            // The loop guard guarantees capacity for this push.
            let _ = entries.push(next);
            cursor = next.sequence;
        }
        JournalPage { entries, latest_sequence: self.next_sequence.wrapping_sub(1) }
    }

    #[must_use]
    pub fn page(&self, after_sequence: u32) -> JournalPage {
        self.page_where(after_sequence, |_| true)
    }

    #[must_use]
    pub fn summary(&self) -> JournalSummary {
        let retained = self.entries.iter().flatten().count();
        let oldest_sequence = self.entries.iter().flatten().map(|entry| entry.sequence).min().unwrap_or(0);
        JournalSummary {
            retained: u8::try_from(retained).unwrap_or(u8::MAX),
            oldest_sequence,
            latest_sequence: self.next_sequence.wrapping_sub(1),
            overwritten: self.overwritten,
        }
    }
}

impl<const N: usize> Default for Journal<N> {
    fn default() -> Self {
        Self::new()
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

    use super::Journal;

    #[test]
    fn overwritten_records_gaps() {
        let mut journal = Journal::<2>::new();
        journal.push(10, request(1));
        journal.push(20, request(2));
        journal.push(30, request(3));

        let oldest = journal.page(0);
        let oldest = *oldest.entries.first().expect("one retained entry");
        assert_eq!(oldest.sequence, 2);
        assert_eq!(oldest.timestamp_ms, 20);

        let latest = journal.page(oldest.sequence);
        let latest = *latest.entries.first().expect("one newer entry");
        assert_eq!(latest.sequence, 3);
        assert_eq!(latest.timestamp_ms, 30);
        assert!(journal.page(latest.sequence).entries.is_empty());
        assert_eq!(journal.summary().retained, 2);
        assert_eq!(journal.summary().oldest_sequence, 2);
        assert_eq!(journal.summary().latest_sequence, 3);
        assert_eq!(journal.summary().overwritten, 1);
    }

    const fn request(id: u32) -> JournalEvent {
        JournalEvent::MeshRequestReceived { peer: NodeId(273_282), request_id: RequestId(id) }
    }
}
