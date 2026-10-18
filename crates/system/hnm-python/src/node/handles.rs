/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node handles bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::time::Duration;

// Third Party Imports
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// Local Imports
use super::{
    PyBootloader, PyDeviceSelector, PyJournalEntry, PyJournalPage, PyNodeDescription, PyNodeDiagnostics, PyNodeId, PyNodeStatus,
    PyOperationId, PyOperationSnapshot,
};
use crate::runtime::awaitable;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "NodeHandle", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeHandle {
    pub(crate) inner: hnm::NodeHandle,
}

#[pyclass(name = "Operation", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyOperation {
    pub(super) inner: hnm::Operation,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyNodeHandle {
    pub(crate) const fn wrap(inner: hnm::NodeHandle) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeHandle {
    #[staticmethod]
    #[pyo3(signature = (selector = None, *, port = None, device = None))]
    fn connect<'py>(
        py: Python<'py>, selector: Option<&PyDeviceSelector>, port: Option<String>, device: Option<&PyNodeId>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let selector = crate::gateway::resolve_selector(selector, port, device)?;
        awaitable(py, async move { hnm::NodeHandle::connect(selector).await.map(Self::wrap) })
    }

    #[getter]
    const fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id())
    }

    #[getter]
    const fn is_remote(&self) -> bool {
        self.inner.is_remote()
    }

    fn with_timeout(&self, seconds: f64) -> PyResult<Self> {
        let timeout = Duration::try_from_secs_f64(seconds).map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self::wrap(self.inner.clone().with_timeout(timeout)))
    }

    fn describe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.describe().await.map(PyNodeDescription::wrap) })
    }

    fn status<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.status().await.map(PyNodeStatus::wrap) })
    }

    fn identify<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.identify().await })
    }

    fn diagnostics<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.diagnostics().await.map(PyNodeDiagnostics::wrap) })
    }

    fn journal_page<'py>(&self, py: Python<'py>, after_sequence: u32) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.journal_page(after_sequence).await.map(|inner| PyJournalPage { inner }) })
    }

    fn journal<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.journal().await.map(|entries| entries.into_iter().map(PyJournalEntry::wrap).collect::<Vec<_>>()) })
    }

    #[pyo3(signature = (bootloader = None))]
    fn restart<'py>(&self, py: Python<'py>, bootloader: Option<&PyBootloader>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let bootloader = bootloader.map(|bootloader| bootloader.inner);
        awaitable(py, async move { node.restart(bootloader).await })
    }

    fn operation_status<'py>(&self, py: Python<'py>, operation: &PyOperationId) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let operation = operation.inner;
        awaitable(py, async move { node.operation_status(operation).await.map(PyOperationSnapshot::wrap) })
    }

    fn poll<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move { node.poll().await })
    }

    fn operation(&self, id: &PyOperationId) -> PyOperation {
        PyOperation { inner: self.inner.operation(id.inner) }
    }

    fn __repr__(&self) -> String {
        format!("NodeHandle(id=NodeId({}), is_remote={})", self.inner.id().0, if self.inner.is_remote() { "True" } else { "False" })
    }
}

impl PyOperation {
    pub(crate) const fn wrap(inner: hnm::Operation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOperation {
    fn cancel<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let operation = self.inner.clone();
        awaitable(py, async move { operation.cancel().await })
    }

    #[getter]
    const fn id(&self) -> PyOperationId {
        PyOperationId::wrap(self.inner.id())
    }

    fn status<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let operation = self.inner.clone();
        awaitable(py, async move { operation.status().await.map(PyOperationSnapshot::wrap) })
    }

    fn completed<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let operation = self.inner.clone();
        awaitable(py, async move { operation.completed().await.map(PyOperationSnapshot::wrap) })
    }

    fn settled<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let operation = self.inner.clone();
        awaitable(py, async move { operation.settled().await.map(PyOperationSnapshot::wrap) })
    }

    fn __repr__(&self) -> String {
        format!("Operation(id=OperationId({}))", self.inner.id().0)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
