/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node status bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::{PyAuxGpioAccess, PyAuxGpioMode, PyChannelId, PyNodeInfo, PyOperationId, PyOperationState, PySatelliteName, PySatelliteState};
use hnm::node::{NodeStatus, OperationSnapshot, SatelliteReport};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "SatelliteReport", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySatelliteReport {
    pub(super) inner: SatelliteReport,
}

#[pyclass(name = "NodeStatus", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeStatus {
    inner: NodeStatus,
}

#[pyclass(name = "OperationSnapshot", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyOperationSnapshot {
    inner: OperationSnapshot,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PySatelliteReport {
    #[getter]
    const fn name(&self) -> PySatelliteName {
        PySatelliteName::wrap(self.inner.name)
    }

    #[getter]
    const fn state(&self) -> PySatelliteState {
        PySatelliteState::wrap(self.inner.state)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PyNodeStatus {
    pub(crate) const fn wrap(inner: NodeStatus) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeStatus {
    #[getter]
    fn info(&self) -> PyNodeInfo {
        PyNodeInfo::wrap(self.inner.info.clone())
    }

    #[getter]
    fn auxiliary_gpio_access(&self) -> Vec<PyAuxGpioAccess> {
        self.inner.auxiliary_gpio_access.into_iter().map(|inner| PyAuxGpioAccess { inner }).collect()
    }

    #[getter]
    fn auxiliary_gpio(&self) -> Vec<PyAuxGpioMode> {
        self.inner.auxiliary_gpio.iter().map(|mode| PyAuxGpioMode::wrap(*mode)).collect()
    }

    fn __repr__(&self) -> String {
        format!("NodeStatus(id=NodeId({}))", self.inner.info.id.0)
    }
}

impl PyOperationSnapshot {
    pub(crate) const fn wrap(inner: OperationSnapshot) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOperationSnapshot {
    #[getter]
    fn error(&self) -> Option<String> {
        self.inner.error.map(|error| error.to_string())
    }
    #[getter]
    fn cleanup_error(&self) -> Option<String> {
        self.inner.cleanup_error.map(|error| error.to_string())
    }
    #[getter]
    const fn id(&self) -> PyOperationId {
        PyOperationId::wrap(self.inner.id)
    }

    #[getter]
    const fn channel(&self) -> PyChannelId {
        PyChannelId::wrap(self.inner.channel)
    }

    #[getter]
    const fn state(&self) -> PyOperationState {
        PyOperationState::wrap(self.inner.state)
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationSnapshot(id=OperationId({}), channel=ChannelId({}), state={:?})",
            self.inner.id.0, self.inner.channel.0, self.inner.state
        )
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
