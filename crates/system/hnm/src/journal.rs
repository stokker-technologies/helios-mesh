/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Versioned JSONL journal record schema and shared journal pagination.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::time::{SystemTime, UNIX_EPOCH};

// Third Party Imports
use serde::Serialize;

// Local Imports
use crate::{
    Result,
    node::{JournalEntry, JournalEvent, JournalPage, NodeHandle, NodeId},
};

/* === Constants === */

pub const SCHEMA_VERSION: u32 = 1;
pub const RECORD_VERSION: u32 = 1;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, Serialize)]
pub struct Header {
    pub schema: u32,

    pub node: u32,

    pub captured_at_us: i64,
}

#[derive(Debug, Serialize)]
pub struct Record<'entry> {
    pub position: u64,

    pub kind: Kind,

    pub id: &'static str,

    pub version: u32,

    pub metadata: Metadata,

    pub payload: &'entry JournalEvent,
}

#[derive(Debug, Serialize)]
pub struct Metadata {
    pub node: u32,

    pub device_sequence: u32,

    pub device_uptime_ms: u64,

    pub observed_at_us: i64,
}

#[derive(Debug, Default)]
pub struct Cursor {
    after_sequence: u32,
    latest: u32,
    caught_up: bool,
}

/* === Enums === */

#[derive(Debug, Serialize)]
pub enum Kind {
    Record,
}

/* === Traits === */

/* === Implementations === */

impl Header {
    #[must_use]
    pub fn new(node: NodeId) -> Self {
        Self { schema: SCHEMA_VERSION, node: node.0, captured_at_us: now_us() }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl<'entry> Record<'entry> {
    #[must_use]
    pub fn new(position: u64, node: NodeId, entry: &'entry JournalEntry) -> Self {
        Self {
            position,
            kind: Kind::Record,
            id: event_name(&entry.event),
            version: RECORD_VERSION,
            metadata: Metadata {
                node: node.0,
                device_sequence: entry.sequence,
                device_uptime_ms: entry.timestamp_ms,
                observed_at_us: now_us(),
            },
            payload: &entry.event,
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl Cursor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn position(&self) -> u32 {
        self.after_sequence
    }

    #[must_use]
    pub const fn latest(&self) -> u32 {
        self.latest
    }

    #[must_use]
    pub const fn is_caught_up(&self) -> bool {
        self.caught_up
    }

    pub async fn next(&mut self, node: &NodeHandle) -> Result<Vec<JournalEntry>> {
        loop {
            let page = node.journal_page(self.after_sequence).await?;
            if let Some(entries) = self.apply(page) {
                return Ok(entries);
            }
        }
    }

    fn apply(&mut self, page: JournalPage) -> Option<Vec<JournalEntry>> {
        if page.latest_sequence < self.after_sequence {
            self.after_sequence = 0;
            return None;
        }
        self.latest = page.latest_sequence;
        let mut entries = Vec::new();
        for entry in page.entries {
            if entry.sequence > self.after_sequence {
                self.after_sequence = entry.sequence;
                entries.push(entry);
            }
        }
        self.caught_up = self.after_sequence >= self.latest;
        Some(entries)
    }
}

/* === Public Functions === */

#[must_use]
pub const fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::MotionAccepted { .. } => "hnm.motion.accepted",
        JournalEvent::CapabilityEventQueued { .. } => "hnm.capability-event.queued",
        JournalEvent::CapabilityEventReturned { .. } => "hnm.capability-event.returned",
        JournalEvent::RequestRetried { .. } => "hnm.request.retried",
        JournalEvent::MeshRoundTrip { .. } => "hnm.mesh.round-trip",
        JournalEvent::MeshRequestReceived { .. } => "hnm.mesh.request-received",
        JournalEvent::MeshResponseSent { .. } => "hnm.mesh.response-sent",
        JournalEvent::MeshTransmit { .. } => "hnm.mesh.transmit",
        JournalEvent::NodeEvicted { .. } => "hnm.node.evicted",
        JournalEvent::SatelliteState { .. } => "hnm.satellite.state",
        JournalEvent::OperationCacheSaturated { .. } => "hnm.operation-cache.saturated",
        JournalEvent::MotionStopped { .. } => "hnm.motion.stopped",
        JournalEvent::MotionCompleted { .. } => "hnm.motion.completed",
        JournalEvent::I2cBaudApplied { .. } => "hnm.i2c.baud-applied",
    }
}

#[must_use]
pub fn now_us() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_micros()).unwrap_or(i64::MAX),
        Err(error) => -i64::try_from(error.duration().as_micros()).unwrap_or(i64::MAX),
    }
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use hnm_protocol::node::RequestId;
    use hnm_protocol::satellite::{SatelliteName, SatelliteState};

    use super::*;

    #[test]
    fn event_names_names() {
        let event = JournalEvent::RequestRetried { peer: NodeId(42), request_id: RequestId(9), retry: 1 };
        assert_eq!(event_name(&event), "hnm.request.retried");
        let event = JournalEvent::SatelliteState { name: SatelliteName::MotorWing, state: SatelliteState::Absent };
        assert_eq!(event_name(&event), "hnm.satellite.state");
        let event = JournalEvent::I2cBaudApplied { frequency_hz: 400_000, hcnt: 126, lcnt: 187, spklen: 11 };
        assert_eq!(event_name(&event), "hnm.i2c.baud-applied");
    }

    #[test]
    fn header_record_lines() {
        let header = Header::new(NodeId(42));
        let line = header.to_json().unwrap();
        assert!(line.contains("\"schema\":1"));
        assert!(line.contains("\"node\":42"));

        let entry = JournalEntry {
            sequence: 3,
            timestamp_ms: 30,
            event: JournalEvent::RequestRetried { peer: NodeId(42), request_id: RequestId(9), retry: 1 },
        };
        let record = Record::new(1, NodeId(42), &entry);
        let line = record.to_json().unwrap();
        assert!(line.contains("\"id\":\"hnm.request.retried\""));
        assert!(line.contains("\"position\":1"));
    }

    fn entry(sequence: u32) -> JournalEntry {
        JournalEntry {
            sequence,
            timestamp_ms: u64::from(sequence) * 10,
            event: JournalEvent::MeshRequestReceived { peer: NodeId(42), request_id: RequestId(sequence) },
        }
    }

    fn page(sequences: &[u32], latest: u32) -> JournalPage {
        let mut entries = heapless::Vec::new();
        for &sequence in sequences {
            entries.push(entry(sequence)).unwrap();
        }
        JournalPage { entries, latest_sequence: latest }
    }

    #[test]
    fn cursor_advances_up() {
        let mut cursor = Cursor::new();
        let batch = cursor.apply(page(&[1, 2], 3)).unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(cursor.position(), 2);
        assert!(!cursor.is_caught_up());
        let batch = cursor.apply(page(&[3], 3)).unwrap();
        assert_eq!(batch.len(), 1);
        assert!(cursor.is_caught_up());
    }

    #[test]
    fn cursor_skips_sequences() {
        let mut cursor = Cursor::new();
        cursor.apply(page(&[1, 2], 2)).unwrap();
        let batch = cursor.apply(page(&[2], 2)).unwrap();
        assert_eq!(batch, [] as [hnm_protocol::diagnostics::JournalEntry; 0]);
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn cursor_resets_restarts() {
        let mut cursor = Cursor::new();
        cursor.apply(page(&[7, 8], 8)).unwrap();
        assert!(cursor.apply(page(&[], 0)).is_none());
        assert_eq!(cursor.position(), 0);
        let batch = cursor.apply(page(&[1], 1)).unwrap();
        assert_eq!(batch.len(), 1);
        assert!(cursor.is_caught_up());
    }
}
