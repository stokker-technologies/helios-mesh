/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node description bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::{PyNodeId, PySatelliteReport};
use hnm::node::{FirmwareVersion, NodeDescription, NodeInfo};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "FirmwareVersion", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyFirmwareVersion {
    pub(super) inner: FirmwareVersion,
}

#[pyclass(name = "NodeInfo", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeInfo {
    inner: NodeInfo,
}

#[pyclass(name = "NodeDescription", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeDescription {
    inner: NodeDescription,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyFirmwareVersion {
    #[getter]
    const fn major(&self) -> u8 {
        self.inner.major
    }

    #[getter]
    const fn minor(&self) -> u8 {
        self.inner.minor
    }

    #[getter]
    const fn patch(&self) -> u8 {
        self.inner.patch
    }

    fn __str__(&self) -> String {
        format!("{}.{}.{}", self.inner.major, self.inner.minor, self.inner.patch)
    }

    fn __repr__(&self) -> String {
        format!("FirmwareVersion({}, {}, {})", self.inner.major, self.inner.minor, self.inner.patch)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PyNodeInfo {
    pub(crate) const fn wrap(inner: NodeInfo) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeInfo {
    #[getter]
    fn node_type(&self) -> crate::discovery::PyNodeType {
        crate::discovery::PyNodeType { inner: self.inner.node_type }
    }

    #[getter]
    const fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id)
    }

    #[getter]
    fn label(&self) -> String {
        self.inner.label().to_owned()
    }

    #[getter]
    fn identifier(&self) -> &'static str {
        self.inner.node_type.definition().identifier
    }

    #[getter]
    fn description(&self) -> &'static str {
        self.inner.node_type.definition().description
    }

    #[getter]
    const fn firmware(&self) -> PyFirmwareVersion {
        PyFirmwareVersion { inner: self.inner.firmware }
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeInfo(id=NodeId({}), label={:?}, firmware={}.{}.{})",
            self.inner.id.0,
            self.inner.label(),
            self.inner.firmware.major,
            self.inner.firmware.minor,
            self.inner.firmware.patch,
        )
    }
}

impl PyNodeDescription {
    pub(crate) const fn wrap(inner: NodeDescription) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeDescription {
    #[getter]
    fn info(&self) -> PyNodeInfo {
        PyNodeInfo::wrap(self.inner.info.clone())
    }

    #[getter]
    fn satellites(&self) -> Vec<PySatelliteReport> {
        self.inner.satellites.iter().map(|satellite| PySatelliteReport { inner: *satellite }).collect()
    }

    fn __repr__(&self) -> String {
        format!("NodeDescription(id=NodeId({}), satellites={})", self.inner.info.id.0, self.inner.satellites.len())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
