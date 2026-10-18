/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Root USB discovery bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{error::map_error, node::PyNodeId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "UsbDevice", module = "helios_mesh", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyUsbDevice {
    inner: helios_mesh::UsbDevice,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyUsbDevice {
    #[getter]
    const fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id)
    }

    #[getter]
    fn label(&self) -> String {
        self.inner.label.clone()
    }

    #[getter]
    fn port(&self) -> String {
        self.inner.port.clone()
    }

    fn __repr__(&self) -> String {
        format!("UsbDevice(id=NodeId({}), label={:?}, port={:?})", self.inner.id.0, self.inner.label, self.inner.port)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/* === Public Functions === */

#[pyfunction]
pub(crate) fn usb_devices() -> PyResult<Vec<PyUsbDevice>> {
    helios_mesh::usb_devices()
        .map(|devices| devices.into_iter().map(|inner| PyUsbDevice { inner }).collect())
        .map_err(|error| map_error(&error))
}

/* === Private Functions === */

/* === Tests === */
