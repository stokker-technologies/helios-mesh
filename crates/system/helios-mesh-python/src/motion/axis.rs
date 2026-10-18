/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python motion axis bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::PyDescription;
use crate::node::PyChannelId;
use crate::node::PyOperation;
use crate::runtime::awaitable;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "Axis", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAxis {
    inner: helios_mesh::Axis,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyAxis {
    pub(super) const fn wrap(inner: helios_mesh::Axis) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAxis {
    #[getter]
    fn id(&self) -> PyChannelId {
        PyChannelId::wrap(self.inner.id())
    }

    #[getter]
    fn description(&self) -> PyDescription {
        PyDescription::wrap(*self.inner.description())
    }

    #[pyo3(signature = (degrees, *, speed = None, acceleration = None))]
    fn turn<'py>(&self, py: Python<'py>, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.turn(degrees, speed, acceleration).await.map(crate::node::PyOperationSnapshot::wrap) })
    }

    #[pyo3(signature = (degrees, *, speed = None, acceleration = None))]
    fn move_to<'py>(&self, py: Python<'py>, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.move_to(degrees, speed, acceleration).await.map(crate::node::PyOperationSnapshot::wrap) })
    }

    fn zero<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.zero().await.map(crate::node::PyOperationSnapshot::wrap) })
    }

    fn stop<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.stop().await.map(crate::node::PyOperationSnapshot::wrap) })
    }

    #[pyo3(signature = (degrees, *, speed = None, acceleration = None))]
    fn start_turn<'py>(&self, py: Python<'py>, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.start_turn(degrees, speed, acceleration).await.map(PyOperation::wrap) })
    }

    #[pyo3(signature = (degrees, *, speed = None, acceleration = None))]
    fn start_move_to<'py>(
        &self, py: Python<'py>, degrees: f64, speed: Option<u16>, acceleration: Option<u16>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.start_move_to(degrees, speed, acceleration).await.map(PyOperation::wrap) })
    }

    fn start_zero<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.start_zero().await.map(PyOperation::wrap) })
    }

    fn start_stop<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let axis = self.inner.clone();
        awaitable(py, async move { axis.start_stop().await.map(PyOperation::wrap) })
    }

    fn __repr__(&self) -> String {
        format!("Axis(id=ChannelId({}))", self.inner.id().0)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
