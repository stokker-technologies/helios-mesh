/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Typed Python USB discovery and node handles.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::node::PyChannelId;
use crate::{
    error::map_error,
    node::{PyAuxGpioId, PyAuxGpioMode, PyDeviceSelector, PyNodeHandle, PyNodeId, PyNodeInfo},
    runtime::awaitable,
};
use helios_mesh::ChannelId;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "NodeType", module = "helios_mesh", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeType {
    pub(crate) inner: helios_mesh::NodeType,
}

#[pyclass(name = "DiscoveredNode", module = "helios_mesh", frozen, skip_from_py_object)]
pub(crate) struct PyDiscoveredNode {
    inner: helios_mesh::DiscoveredNode,
}

#[pyclass(name = "SidecarNode", module = "helios_mesh", frozen, skip_from_py_object)]
pub(crate) struct PySidecarNode {
    inner: helios_mesh::SidecarNode,
}

#[pyclass(name = "RotatorNode", module = "helios_mesh", frozen, skip_from_py_object)]
pub(crate) struct PyRotatorNode {
    inner: helios_mesh::RotatorNode,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[pymethods]
impl PyNodeType {
    #[classattr]
    const SIDECAR: Self = Self { inner: helios_mesh::NodeType::Sidecar };
    #[classattr]
    const GATEWAY: Self = Self { inner: helios_mesh::NodeType::Gateway };
    #[classattr]
    const ROTATOR: Self = Self { inner: helios_mesh::NodeType::Rotator };
    #[getter]
    fn identifier(&self) -> &'static str {
        self.inner.definition().identifier
    }
    #[getter]
    fn label(&self) -> &'static str {
        self.inner.definition().label
    }
    #[getter]
    fn description(&self) -> &'static str {
        self.inner.definition().description
    }
    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
    fn __repr__(&self) -> String {
        format!("NodeType({:?})", self.inner.definition().identifier)
    }
}

#[pymethods]
impl PyDiscoveredNode {
    #[getter]
    fn info(&self) -> PyNodeInfo {
        PyNodeInfo::wrap(self.inner.info.clone())
    }
    #[getter]
    fn node_type(&self) -> PyNodeType {
        PyNodeType { inner: self.inner.info.node_type }
    }
    #[getter]
    fn port(&self) -> String {
        self.inner.port.clone()
    }
    fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let entry = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let node = entry.connect().await.map_err(|error| map_error(&error))?;
            Python::attach(|py| wrap_connected(py, node))
        })
    }
}

#[pymethods]
impl PySidecarNode {
    #[staticmethod]
    fn connect<'py>(py: Python<'py>, selector: &PyDeviceSelector) -> PyResult<Bound<'py, PyAny>> {
        let selector = selector.inner.clone();
        awaitable(py, async move { helios_mesh::SidecarNode::connect(selector).await.map(|inner| Self { inner }) })
    }
    #[getter]
    fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id())
    }
    #[getter]
    fn node(&self) -> PyNodeHandle {
        PyNodeHandle::wrap(self.inner.node().clone())
    }
    fn set_gpio<'py>(&self, py: Python<'py>, gpio: &PyAuxGpioId, mode: &PyAuxGpioMode) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let gpio = gpio.inner;
        let mode = mode.inner;
        awaitable(py, async move { node.set_gpio(gpio, mode).await })
    }
    fn pulse_gpio<'py>(&self, py: Python<'py>, gpio: &PyAuxGpioId, duration: std::time::Duration) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let gpio = gpio.inner;
        awaitable(py, async move { node.pulse_gpio(gpio, duration).await.map(crate::node::PyOperation::wrap) })
    }
}

#[pymethods]
impl PyRotatorNode {
    fn channels<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        awaitable(py, async move {
            node.channels().await.map(|channels| channels.into_iter().map(crate::motion::PyDescription::wrap).collect::<Vec<_>>())
        })
    }
    #[pyo3(signature = (channels = None))]
    fn axes<'py>(&self, py: Python<'py>, channels: Option<Vec<Py<PyChannelId>>>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let channels: Option<Vec<ChannelId>> = channels.map(|ids| ids.iter().map(|id| id.borrow(py).inner).collect());
        awaitable(py, async move { node.axes(channels.as_deref()).await.map(crate::motion::PyAxesGuard::wrap) })
    }
    #[pyo3(signature = (*steppers))]
    fn configure<'py>(&self, py: Python<'py>, steppers: Vec<Py<crate::motion::PyStepperConfig>>) -> PyResult<Bound<'py, PyAny>> {
        let node = self.inner.clone();
        let steppers: Vec<helios_mesh::StepperConfig> = steppers.into_iter().map(|config| config.borrow(py).inner).collect();
        awaitable(py, async move { node.configure(&steppers).await })
    }

    #[staticmethod]
    fn connect<'py>(py: Python<'py>, selector: &PyDeviceSelector) -> PyResult<Bound<'py, PyAny>> {
        let selector = selector.inner.clone();
        awaitable(py, async move { helios_mesh::RotatorNode::connect(selector).await.map(|inner| Self { inner }) })
    }
    #[getter]
    fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id())
    }
    #[getter]
    fn node(&self) -> PyNodeHandle {
        PyNodeHandle::wrap(self.inner.node().clone())
    }
}

/* === Public Functions === */

#[pyfunction]
#[pyo3(signature = (node_type = None))]
pub(crate) fn discover<'py>(py: Python<'py>, node_type: Option<&PyNodeType>) -> PyResult<Bound<'py, PyAny>> {
    let kind = node_type.map(|value| value.inner);
    awaitable(py, async move {
        let nodes = match kind {
            Some(kind) => helios_mesh::discover_type(kind).await?,
            None => helios_mesh::discover().await?,
        };
        Ok(nodes.into_iter().map(|inner| PyDiscoveredNode { inner }).collect::<Vec<_>>())
    })
}

pub(crate) fn wrap_connected(py: Python<'_>, node: helios_mesh::ConnectedNode) -> PyResult<Py<PyAny>> {
    match node {
        helios_mesh::ConnectedNode::Sidecar(inner) => Py::new(py, PySidecarNode { inner }).map(Py::into_any),
        helios_mesh::ConnectedNode::Gateway(inner) => Py::new(py, crate::gateway::PyGatewayNode::wrap(inner)).map(Py::into_any),
        helios_mesh::ConnectedNode::Rotator(inner) => Py::new(py, PyRotatorNode { inner }).map(Py::into_any),
    }
}

/* === Private Functions === */

/* === Tests === */
