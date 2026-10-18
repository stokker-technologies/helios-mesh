/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node identity bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// Local Imports
use helios_mesh::node::{AuxGpioId, ChannelId, NodeId, OperationId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "NodeId", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeId {
    pub(crate) inner: NodeId,
}

#[pyclass(name = "ChannelId", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyChannelId {
    pub(crate) inner: ChannelId,
}

#[pyclass(name = "OperationId", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyOperationId {
    pub(crate) inner: OperationId,
}

#[pyclass(name = "AuxGpioId", module = "helios_mesh.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAuxGpioId {
    pub(crate) inner: AuxGpioId,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyNodeId {
    pub(crate) const fn wrap(inner: NodeId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeId {
    #[new]
    const fn new(value: u32) -> Self {
        Self::wrap(NodeId(value))
    }

    #[getter]
    const fn value(&self) -> u32 {
        self.inner.0
    }

    const fn is_device(&self) -> bool {
        self.inner.is_device()
    }

    const fn __int__(&self) -> u32 {
        self.inner.0
    }

    fn __str__(&self) -> String {
        format!("{:06}", self.inner.0)
    }

    fn __repr__(&self) -> String {
        format!("NodeId({})", self.inner.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    const fn __hash__(&self) -> u64 {
        self.inner.0 as u64
    }
}

impl PyChannelId {
    pub(crate) const fn wrap(inner: ChannelId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyChannelId {
    #[new]
    fn new(value: u8) -> PyResult<Self> {
        if value == 0 {
            return Err(PyValueError::new_err("channel IDs are one-based"));
        }
        Ok(Self::wrap(ChannelId(value)))
    }

    #[getter]
    const fn value(&self) -> u8 {
        self.inner.0
    }

    const fn __int__(&self) -> u8 {
        self.inner.0
    }

    fn __repr__(&self) -> String {
        format!("ChannelId({})", self.inner.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    const fn __hash__(&self) -> u64 {
        self.inner.0 as u64
    }
}

impl PyOperationId {
    pub(crate) const fn wrap(inner: OperationId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOperationId {
    #[new]
    const fn new(value: u32) -> Self {
        Self::wrap(OperationId(value))
    }

    #[getter]
    const fn value(&self) -> u32 {
        self.inner.0
    }

    const fn __int__(&self) -> u32 {
        self.inner.0
    }

    fn __repr__(&self) -> String {
        format!("OperationId({})", self.inner.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    const fn __hash__(&self) -> u64 {
        self.inner.0 as u64
    }
}

#[pymethods]
impl PyAuxGpioId {
    #[new]
    fn new(value: u8) -> PyResult<Self> {
        let inner = AuxGpioId(value);
        if !inner.is_valid() {
            return Err(PyValueError::new_err("auxiliary GPIO IDs are 1 through 4"));
        }
        Ok(Self { inner })
    }

    #[getter]
    const fn value(&self) -> u8 {
        self.inner.0
    }

    const fn __int__(&self) -> u8 {
        self.inner.0
    }

    fn __repr__(&self) -> String {
        format!("AuxGpioId({})", self.inner.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    const fn __hash__(&self) -> u64 {
        self.inner.0 as u64
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
