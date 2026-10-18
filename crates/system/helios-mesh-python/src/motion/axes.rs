/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python motion axes bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::sync::Arc;

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::PyAxis;
use crate::node::PyOperation;
use crate::runtime::awaitable;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "AxesGuard", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAxesGuard {
    inner: Arc<helios_mesh::AxesGuard>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyAxesGuard {
    pub(crate) fn wrap(inner: helios_mesh::AxesGuard) -> Self {
        Self { inner: Arc::new(inner) }
    }
}

#[pymethods]
impl PyAxesGuard {
    #[getter]
    fn axes(&self) -> Vec<PyAxis> {
        self.inner.axes().iter().cloned().map(PyAxis::wrap).collect()
    }

    fn turn_together<'py>(&self, py: Python<'py>, degrees: Vec<f64>) -> PyResult<Bound<'py, PyAny>> {
        let guard = self.inner.clone();
        awaitable(py, async move {
            let snapshots = guard.turn_together(&degrees).await?;
            Ok(snapshots.into_iter().map(crate::node::PyOperationSnapshot::wrap).collect::<Vec<_>>())
        })
    }

    fn start_turn_together<'py>(&self, py: Python<'py>, degrees: Vec<f64>) -> PyResult<Bound<'py, PyAny>> {
        let guard = self.inner.clone();
        awaitable(py, async move {
            let operations = guard.start_turn_together(&degrees).await?;
            Ok(operations.into_iter().map(PyOperation::wrap).collect::<Vec<_>>())
        })
    }

    fn __aenter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let axes = self.axes();
        awaitable(py, async move { Ok::<_, helios_mesh::Error>(axes) })
    }

    fn __aexit__<'py>(
        &self, py: Python<'py>, exc_type: &Bound<'py, PyAny>, exc_value: &Bound<'py, PyAny>, traceback: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = (self, exc_type, exc_value, traceback);
        awaitable(py, async move { Ok::<_, helios_mesh::Error>(false) })
    }

    fn __repr__(&self) -> String {
        format!("AxesGuard(axes={})", self.inner.axes().len())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
