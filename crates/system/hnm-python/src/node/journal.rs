/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node journal bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::{PyChannelId, PyNodeId, PyOperationId};
use hnm::node::{JournalEntry, JournalEvent, JournalPage};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "JournalEvent", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyJournalEvent {
    pub(crate) inner: JournalEvent,
}

#[pyclass(name = "JournalEntry", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyJournalEntry {
    pub(crate) inner: JournalEntry,
}

#[pyclass(name = "JournalPage", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyJournalPage {
    pub(super) inner: JournalPage,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyJournalEvent {
    #[staticmethod]
    const fn i2c_baud_applied(frequency_hz: u32, hcnt: u16, lcnt: u16, spklen: u8) -> Self {
        Self { inner: JournalEvent::I2cBaudApplied { frequency_hz, hcnt, lcnt, spklen } }
    }

    #[getter]
    const fn name(&self) -> &'static str {
        hnm::journal::event_name(&self.inner)
    }

    #[getter]
    const fn peer(&self) -> Option<PyNodeId> {
        match self.inner {
            JournalEvent::MotionAccepted { peer, .. }
            | JournalEvent::CapabilityEventQueued { peer, .. }
            | JournalEvent::CapabilityEventReturned { peer, .. }
            | JournalEvent::RequestRetried { peer, .. }
            | JournalEvent::MeshRoundTrip { peer, .. }
            | JournalEvent::MeshRequestReceived { peer, .. }
            | JournalEvent::MeshResponseSent { peer, .. }
            | JournalEvent::MeshTransmit { peer, .. } => Some(PyNodeId::wrap(peer)),
            JournalEvent::NodeEvicted { node } | JournalEvent::OperationCacheSaturated { node, .. } => Some(PyNodeId::wrap(node)),
            JournalEvent::SatelliteState { .. }
            | JournalEvent::MotionStopped { .. }
            | JournalEvent::MotionCompleted { .. }
            | JournalEvent::I2cBaudApplied { .. } => None,
        }
    }

    #[getter]
    const fn request_id(&self) -> Option<u32> {
        match self.inner {
            JournalEvent::MotionAccepted { request_id, .. }
            | JournalEvent::CapabilityEventReturned { request_id, .. }
            | JournalEvent::RequestRetried { request_id, .. }
            | JournalEvent::MeshRoundTrip { request_id, .. }
            | JournalEvent::MeshRequestReceived { request_id, .. }
            | JournalEvent::MeshResponseSent { request_id, .. }
            | JournalEvent::MeshTransmit { request_id, .. } => Some(request_id.0),
            JournalEvent::CapabilityEventQueued { .. }
            | JournalEvent::NodeEvicted { .. }
            | JournalEvent::SatelliteState { .. }
            | JournalEvent::OperationCacheSaturated { .. }
            | JournalEvent::MotionStopped { .. }
            | JournalEvent::MotionCompleted { .. }
            | JournalEvent::I2cBaudApplied { .. } => None,
        }
    }

    #[getter]
    const fn channel(&self) -> Option<PyChannelId> {
        match self.inner {
            JournalEvent::MotionAccepted { channel, .. }
            | JournalEvent::MotionStopped { channel, .. }
            | JournalEvent::MotionCompleted { channel, .. } => Some(PyChannelId::wrap(channel)),
            _ => None,
        }
    }

    #[getter]
    const fn operation(&self) -> Option<PyOperationId> {
        match self.inner {
            JournalEvent::MotionAccepted { operation, .. }
            | JournalEvent::OperationCacheSaturated { operation, .. }
            | JournalEvent::MotionStopped { operation, .. }
            | JournalEvent::MotionCompleted { operation, .. } => Some(PyOperationId::wrap(operation)),
            _ => None,
        }
    }

    #[getter]
    const fn requested(&self) -> Option<crate::motion::PyTarget> {
        match self.inner {
            JournalEvent::MotionCompleted { requested, .. } => Some(crate::motion::PyTarget::wrap(requested)),
            _ => None,
        }
    }

    #[getter]
    const fn actual(&self) -> Option<crate::motion::PyTarget> {
        match self.inner {
            JournalEvent::MotionCompleted { actual, .. } => Some(crate::motion::PyTarget::wrap(actual)),
            _ => None,
        }
    }

    const fn involves(&self, node: &PyNodeId) -> bool {
        self.inner.involves(node.inner)
    }

    #[getter]
    fn timing(&self) -> Option<crate::motion::PyTiming> {
        hnm::motion::timing_of(&self.inner).map(crate::motion::PyTiming::wrap)
    }

    #[getter]
    const fn frequency_hz(&self) -> Option<u32> {
        match self.inner {
            JournalEvent::I2cBaudApplied { frequency_hz, .. } => Some(frequency_hz),
            _ => None,
        }
    }

    #[getter]
    const fn hcnt(&self) -> Option<u16> {
        match self.inner {
            JournalEvent::I2cBaudApplied { hcnt, .. } => Some(hcnt),
            _ => None,
        }
    }

    #[getter]
    const fn lcnt(&self) -> Option<u16> {
        match self.inner {
            JournalEvent::I2cBaudApplied { lcnt, .. } => Some(lcnt),
            _ => None,
        }
    }

    #[getter]
    const fn spklen(&self) -> Option<u8> {
        match self.inner {
            JournalEvent::I2cBaudApplied { spklen, .. } => Some(spklen),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl PyJournalEntry {
    pub(crate) const fn wrap(inner: JournalEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyJournalEntry {
    #[getter]
    const fn sequence(&self) -> u32 {
        self.inner.sequence
    }

    #[getter]
    const fn timestamp_ms(&self) -> u64 {
        self.inner.timestamp_ms
    }

    #[getter]
    const fn event(&self) -> PyJournalEvent {
        PyJournalEvent { inner: self.inner.event }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pymethods]
impl PyJournalPage {
    #[getter]
    fn entries(&self) -> Vec<PyJournalEntry> {
        self.inner.entries.iter().map(|entry| PyJournalEntry::wrap(*entry)).collect()
    }

    #[getter]
    const fn latest_sequence(&self) -> u32 {
        self.inner.latest_sequence
    }

    fn __repr__(&self) -> String {
        format!("JournalPage(entries={}, latest_sequence={})", self.inner.entries.len(), self.inner.latest_sequence)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
