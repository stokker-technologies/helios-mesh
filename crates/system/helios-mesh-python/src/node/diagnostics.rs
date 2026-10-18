/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node diagnostics bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::PyNodeId;
use helios_mesh::node::{JournalSummary, NodeDiagnostics, RequestDiagnostics};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "RequestDiagnostics", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyRequestDiagnostics {
    pub(super) inner: RequestDiagnostics,
}

#[pyclass(name = "JournalSummary", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyJournalSummary {
    pub(super) inner: JournalSummary,
}

#[pyclass(name = "NodeDiagnostics", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeDiagnostics {
    inner: NodeDiagnostics,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyRequestDiagnostics {
    #[getter]
    const fn received(&self) -> u32 {
        self.inner.received
    }

    #[getter]
    const fn succeeded(&self) -> u32 {
        self.inner.succeeded
    }

    #[getter]
    const fn failed(&self) -> u32 {
        self.inner.failed
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pymethods]
impl PyJournalSummary {
    #[getter]
    const fn retained(&self) -> u8 {
        self.inner.retained
    }

    #[getter]
    const fn oldest_sequence(&self) -> u32 {
        self.inner.oldest_sequence
    }

    #[getter]
    const fn latest_sequence(&self) -> u32 {
        self.inner.latest_sequence
    }

    #[getter]
    const fn overwritten(&self) -> u32 {
        self.inner.overwritten
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl PyNodeDiagnostics {
    pub(crate) const fn wrap(inner: NodeDiagnostics) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeDiagnostics {
    #[getter]
    const fn node(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.node)
    }

    #[getter]
    const fn uptime_ms(&self) -> u64 {
        self.inner.uptime_ms
    }

    #[getter]
    const fn requests(&self) -> PyRequestDiagnostics {
        PyRequestDiagnostics { inner: self.inner.requests }
    }

    #[getter]
    const fn journal(&self) -> PyJournalSummary {
        PyJournalSummary { inner: self.inner.journal }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
