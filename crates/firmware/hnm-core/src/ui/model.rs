/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Display-facing read model with no hardware ownership.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use super::RequestActivity;
use crate::node::StatusRows;
use hnm_protocol::node::{NodeId, NodeLabel};

/* === Constants === */

pub const MAX_NEARBY_DEVICES: usize = crate::node::services::peers::MAX_NEARBY_DEVICES;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoundTripStats {
    pub last_ms: u32,

    pub mean_ms: u32,

    pub samples: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoundTripWindow {
    samples: [u32; Self::CAPACITY],
    next: usize,
    len: usize,
    sum: u64,
    last_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPage {
    pub id: NodeId,

    pub label: &'static str,

    pub rows: StatusRows,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NearbyPage {
    pub id: NodeId,

    pub label: NodeLabel,

    pub last_seen_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub local: LocalPage,

    pub activity: Option<RequestActivity>,

    pub nearby: heapless::Vec<NearbyPage, MAX_NEARBY_DEVICES>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl RoundTripWindow {
    pub const CAPACITY: usize = 16;

    #[must_use]
    pub const fn new() -> Self {
        Self { samples: [0; Self::CAPACITY], next: 0, len: 0, sum: 0, last_ms: 0 }
    }

    pub fn record(&mut self, duration_ms: u32) {
        if self.len == Self::CAPACITY {
            self.sum -= u64::from(self.samples[self.next]);
        } else {
            self.len += 1;
        }
        self.samples[self.next] = duration_ms;
        self.sum += u64::from(duration_ms);
        self.last_ms = duration_ms;
        self.next = (self.next + 1) % Self::CAPACITY;
    }

    #[must_use]
    pub fn stats(&self) -> Option<RoundTripStats> {
        (self.len > 0).then(|| RoundTripStats {
            last_ms: self.last_ms,
            mean_ms: u32::try_from(self.sum / self.len as u64).unwrap_or(u32::MAX),
            samples: u8::try_from(self.len).unwrap_or(u8::MAX),
        })
    }
}

impl Default for RoundTripWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalPage {
    #[must_use]
    pub const fn new(id: NodeId, label: &'static str) -> Self {
        Self { id, label, rows: StatusRows::new() }
    }
}

impl Snapshot {
    #[must_use]
    pub const fn new(local: LocalPage) -> Self {
        Self { local, activity: None, nearby: heapless::Vec::new() }
    }

    #[must_use]
    pub fn nearby_activity(&self, device: NodeId) -> Option<RequestActivity> {
        self.activity.filter(|activity| activity.peer(self.local.id) == Some(device))
    }

    pub fn upsert_nearby(&mut self, page: NearbyPage) {
        if let Some(slot) = self.nearby.iter_mut().find(|current| current.id == page.id) {
            *slot = page;
            return;
        }
        let _ = self.nearby.push(page);
    }

    #[must_use]
    pub fn nearby_count(&self) -> usize {
        self.nearby.len()
    }

    #[must_use]
    pub fn nearby(&self, id: NodeId) -> Option<&NearbyPage> {
        self.nearby.iter().find(|page| page.id == id)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    fn nearby(id: u32) -> NearbyPage {
        NearbyPage { id: NodeId(id), label: NodeLabel::new(), last_seen_ms: 0 }
    }

    #[test]
    fn rolling_mean_sample() {
        let mut window = RoundTripWindow::new();
        for duration in 1..=17 {
            window.record(duration);
        }
        assert_eq!(window.stats().unwrap().last_ms, 17);
        assert_eq!(window.stats().unwrap().mean_ms, 9);
        assert_eq!(window.stats().unwrap().samples, 16);
    }

    #[test]
    fn upsert_replaces_identity() {
        let mut snapshot = Snapshot::new(LocalPage::new(NodeId(100_000), "Gateway"));
        snapshot.upsert_nearby(nearby(200_000));
        snapshot.upsert_nearby(nearby(200_000));

        assert_eq!(snapshot.nearby_count(), 1);
    }
}
