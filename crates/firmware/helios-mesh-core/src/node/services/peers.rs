/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Bounded nearby-device observations shared by both mesh modes.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh_protocol::node::{NodeId, NodeLabel};

/* === Constants === */

pub const MAX_NEARBY_DEVICES: usize = 16;
pub const NEARBY_TIMEOUT_MS: u64 = 60_000;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub id: NodeId,

    pub label: NodeLabel,

    pub last_seen_ms: u64,
}

pub struct Peers<const N: usize = MAX_NEARBY_DEVICES> {
    entries: [Option<Record>; N],
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Record {
    #[must_use]
    pub fn sighted(id: NodeId, now_ms: u64) -> Self {
        Self { id, label: NodeLabel::new(), last_seen_ms: now_ms }
    }
}

impl<const N: usize> Peers<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: [const { None }; N] }
    }

    pub fn observe(&mut self, id: NodeId, now_ms: u64) {
        if let Some(record) = self.find_mut(id) {
            record.last_seen_ms = now_ms;
            return;
        }
        self.insert(Record::sighted(id, now_ms));
    }

    pub fn update(&mut self, record: Record) {
        if let Some(existing) = self.find_mut(record.id) {
            *existing = record;
            return;
        }
        self.insert(record);
    }

    pub fn expire(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        for slot in &mut self.entries {
            if slot.as_ref().is_some_and(|record| now_ms.saturating_sub(record.last_seen_ms) >= NEARBY_TIMEOUT_MS) {
                *slot = None;
                changed = true;
            }
        }
        changed
    }

    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.entries.iter().flatten()
    }

    fn find_mut(&mut self, id: NodeId) -> Option<&mut Record> {
        self.entries.iter_mut().flatten().find(|record| record.id == id)
    }

    fn insert(&mut self, record: Record) {
        if let Some(slot) = self.entries.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(record);
        }
    }
}

impl<const N: usize> Default for Peers<N> {
    fn default() -> Self {
        Self::new()
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sightings_refresh_descriptions() {
        let mut peers = Peers::<4>::new();
        let mut record = Record::sighted(NodeId(200_000), 10);
        record.label = NodeLabel::try_from("Rotator").unwrap();
        peers.update(record);

        peers.observe(NodeId(200_000), 20);

        let record = peers.iter().next().unwrap();
        assert_eq!(record.label.as_str(), "Rotator");
        assert_eq!(record.last_seen_ms, 20);
    }

    #[test]
    fn stale_observations_expire() {
        let mut peers = Peers::<4>::new();
        peers.observe(NodeId(200_000), 0);
        peers.observe(NodeId(300_000), NEARBY_TIMEOUT_MS / 2);

        assert!(peers.expire(NEARBY_TIMEOUT_MS));
        let remaining: heapless::Vec<NodeId, 4> = peers.iter().map(|record| record.id).collect();
        assert_eq!(remaining.as_slice(), &[NodeId(300_000)]);
        assert!(!peers.expire(NEARBY_TIMEOUT_MS));
    }
}
