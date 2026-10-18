/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node selection bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use super::PyNodeId;
use helios_mesh::node::DeviceSelector;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "DeviceSelector", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyDeviceSelector {
    pub(crate) inner: DeviceSelector,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyDeviceSelector {
    #[staticmethod]
    const fn only() -> Self {
        Self { inner: DeviceSelector::Only }
    }

    #[staticmethod]
    const fn id(id: &PyNodeId) -> Self {
        Self { inner: DeviceSelector::Id(id.inner) }
    }

    #[staticmethod]
    const fn port(port: String) -> Self {
        Self { inner: DeviceSelector::Port(port) }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            DeviceSelector::Only => "DeviceSelector.only()".to_owned(),
            DeviceSelector::Id(id) => format!("DeviceSelector.id(NodeId({}))", id.0),
            DeviceSelector::Port(port) => format!("DeviceSelector.port({port:?})"),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
