/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python journal wrappers.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::sync::Arc;

// Third Party Imports
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use tokio::sync::Mutex;

// Local Imports
use crate::{
    node::{PyJournalEntry, PyJournalEvent, PyNodeHandle, PyNodeId},
    runtime::awaitable,
};
use helios_mesh::journal::{self, Cursor, Header};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "Header", module = "helios_mesh.journal", frozen, skip_from_py_object)]
pub(crate) struct PyHeader {
    inner: Header,
}

#[pyclass(name = "Metadata", module = "helios_mesh.journal", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyMetadata {
    inner: Arc<journal::Metadata>,
}

#[pyclass(name = "Record", module = "helios_mesh.journal", frozen, skip_from_py_object)]
pub(crate) struct PyRecord {
    inner: helios_mesh::node::JournalEntry,
    position: u64,
    metadata: Arc<journal::Metadata>,
}

#[pyclass(name = "Cursor", module = "helios_mesh.journal", frozen, skip_from_py_object)]
pub(crate) struct PyCursor {
    inner: Arc<Mutex<Cursor>>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyHeader {
    #[new]
    fn new(node: &PyNodeId) -> Self {
        Self { inner: Header::new(node.inner) }
    }

    #[getter]
    const fn schema(&self) -> u32 {
        self.inner.schema
    }

    #[getter]
    const fn node(&self) -> u32 {
        self.inner.node
    }

    #[getter]
    const fn captured_at_us(&self) -> i64 {
        self.inner.captured_at_us
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pymethods]
impl PyMetadata {
    #[getter]
    fn node(&self) -> u32 {
        self.inner.node
    }

    #[getter]
    fn device_sequence(&self) -> u32 {
        self.inner.device_sequence
    }

    #[getter]
    fn device_uptime_ms(&self) -> u64 {
        self.inner.device_uptime_ms
    }

    #[getter]
    fn observed_at_us(&self) -> i64 {
        self.inner.observed_at_us
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pymethods]
impl PyRecord {
    #[new]
    fn new(position: u64, node: &PyNodeId, entry: &PyJournalEntry) -> Self {
        let entry = entry.inner;
        let record = journal::Record::new(position, node.inner, &entry);
        Self { position: record.position, metadata: Arc::new(record.metadata), inner: entry }
    }

    #[getter]
    const fn position(&self) -> u64 {
        self.position
    }

    #[getter]
    // An instance property in Python even though every record shares it.
    #[allow(clippy::unused_self)]
    const fn kind(&self) -> &'static str {
        "Record"
    }

    #[getter]
    const fn id(&self) -> &'static str {
        journal::event_name(&self.inner.event)
    }

    #[getter]
    // An instance property in Python even though every record shares it.
    #[allow(clippy::unused_self)]
    const fn version(&self) -> u32 {
        journal::RECORD_VERSION
    }

    #[getter]
    fn metadata(&self) -> PyMetadata {
        PyMetadata { inner: self.metadata.clone() }
    }

    #[getter]
    const fn payload(&self) -> PyJournalEvent {
        PyJournalEvent { inner: self.inner.event }
    }

    fn to_json(&self) -> PyResult<String> {
        let record = journal::Record {
            position: self.position,
            kind: journal::Kind::Record,
            id: self.id(),
            version: self.version(),
            metadata: journal::Metadata {
                node: self.metadata.node,
                device_sequence: self.metadata.device_sequence,
                device_uptime_ms: self.metadata.device_uptime_ms,
                observed_at_us: self.metadata.observed_at_us,
            },
            payload: &self.inner.event,
        };
        serde_json::to_string(&record).map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Record(position={}, id={:?}, version={})", self.position, self.id(), self.version())
    }
}

#[pymethods]
impl PyCursor {
    #[new]
    fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(Cursor::new())) }
    }

    #[getter]
    fn position(&self) -> u32 {
        self.inner.blocking_lock().position()
    }

    #[getter]
    fn latest(&self) -> u32 {
        self.inner.blocking_lock().latest()
    }

    #[getter]
    fn is_caught_up(&self) -> bool {
        self.inner.blocking_lock().is_caught_up()
    }

    fn next<'py>(&self, py: Python<'py>, node: &PyNodeHandle) -> PyResult<Bound<'py, PyAny>> {
        let cursor = self.inner.clone();
        let node = node.inner.clone();
        awaitable(py, async move {
            let mut cursor = cursor.lock().await;
            cursor.next(&node).await.map(|entries| entries.into_iter().map(PyJournalEntry::wrap).collect::<Vec<_>>())
        })
    }

    fn __repr__(&self) -> String {
        let cursor = self.inner.blocking_lock();
        format!(
            "Cursor(position={}, latest={}, is_caught_up={})",
            cursor.position(),
            cursor.latest(),
            if cursor.is_caught_up() { "True" } else { "False" }
        )
    }
}

/* === Public Functions === */

#[pyfunction]
pub(super) const fn event_name(event: &PyJournalEvent) -> &'static str {
    journal::event_name(&event.inner)
}

#[pyfunction]
pub(super) fn now_us() -> i64 {
    journal::now_us()
}

/* === Private Functions === */

/* === Tests === */
