/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python gateway wrappers.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::{exceptions::PyValueError, prelude::*};

// Local Imports
use crate::{
    node::{PyDeviceSelector, PyNodeHandle, PyNodeId, PyNodeInfo},
    runtime::awaitable,
};
use hnm::{
    gateway::{NodePage, NodeSummary, NodeTarget},
    node::DeviceSelector,
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "NodeSummary", module = "hnm.gateway", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodeSummary {
    inner: NodeSummary,
}

#[pyclass(name = "NodePage", module = "hnm.gateway", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyNodePage {
    inner: NodePage,
}

#[pyclass(name = "GatewayNode", module = "hnm.gateway", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyGatewayNode {
    inner: hnm::GatewayNode,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyNodeSummary {
    const fn wrap(inner: NodeSummary) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNodeSummary {
    #[getter]
    fn info(&self) -> PyNodeInfo {
        PyNodeInfo::wrap(self.inner.info.clone())
    }

    #[getter]
    const fn silent_for_s(&self) -> u16 {
        self.inner.silent_for_s
    }

    fn __repr__(&self) -> String {
        format!("NodeSummary(id=NodeId({}), silent_for_s={})", self.inner.info.id.0, self.inner.silent_for_s)
    }
}

#[pymethods]
impl PyNodePage {
    #[getter]
    fn nodes(&self) -> Vec<PyNodeSummary> {
        self.inner.nodes.iter().map(|summary| PyNodeSummary::wrap(summary.clone())).collect()
    }

    #[getter]
    const fn next_after(&self) -> Option<PyNodeId> {
        match self.inner.next_after {
            Some(id) => Some(PyNodeId::wrap(id)),
            None => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("NodePage(nodes={}, next_after={:?})", self.inner.nodes.len(), self.inner.next_after)
    }
}

impl PyGatewayNode {
    pub(crate) const fn wrap(inner: hnm::GatewayNode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyGatewayNode {
    #[staticmethod]
    #[pyo3(signature = (selector = None, *, port = None, device = None))]
    fn connect<'py>(
        py: Python<'py>, selector: Option<&PyDeviceSelector>, port: Option<String>, device: Option<&PyNodeId>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let selector = resolve_selector(selector, port, device)?;
        awaitable(py, async move { hnm::GatewayNode::connect(selector).await.map(Self::wrap) })
    }

    #[getter]
    const fn id(&self) -> PyNodeId {
        PyNodeId::wrap(self.inner.id())
    }

    #[getter]
    fn node(&self) -> PyNodeHandle {
        PyNodeHandle::wrap(self.inner.node().clone())
    }

    #[pyo3(signature = (after = None))]
    fn scan_page<'py>(&self, py: Python<'py>, after: Option<&PyNodeId>) -> PyResult<Bound<'py, PyAny>> {
        let gateway = self.inner.clone();
        let after = after.map(|id| id.inner);
        awaitable(py, async move { gateway.scan_page(after).await.map(|inner| PyNodePage { inner }) })
    }

    fn scan<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let gateway = self.inner.clone();
        awaitable(py, async move { gateway.scan().await.map(|nodes| nodes.into_iter().map(PyNodeSummary::wrap).collect::<Vec<_>>()) })
    }

    #[pyo3(signature = (node_type = None))]
    fn scan_one<'py>(&self, py: Python<'py>, node_type: Option<&crate::discovery::PyNodeType>) -> PyResult<Bound<'py, PyAny>> {
        let gateway = self.inner.clone();
        let kind = node_type.map(|kind| kind.inner);
        awaitable(py, async move { gateway.scan_one(kind).await.map(PyNodeSummary::wrap) })
    }

    fn connect_node<'py>(&self, py: Python<'py>, target: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let target = if let Ok(summary) = target.extract::<PyRef<'_, PyNodeSummary>>() {
            NodeTarget::from(summary.inner.clone())
        } else if let Ok(id) = target.extract::<PyRef<'_, PyNodeId>>() {
            NodeTarget::from(id.inner)
        } else {
            return Err(PyValueError::new_err("connect_node takes a NodeSummary or a NodeId"));
        };
        let gateway = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let node = gateway.connect_node(target).await.map_err(|error| crate::error::map_error(&error))?;
            Python::attach(|py| crate::discovery::wrap_connected(py, node))
        })
    }

    fn remote(&self, id: &PyNodeId) -> PyNodeHandle {
        PyNodeHandle::wrap(self.inner.remote(id.inner))
    }

    fn __repr__(&self) -> String {
        format!("Gateway(id=NodeId({}))", self.inner.id().0)
    }
}

/* === Public Functions === */

pub(crate) fn resolve_selector(
    selector: Option<&PyDeviceSelector>, port: Option<String>, device: Option<&PyNodeId>,
) -> PyResult<DeviceSelector> {
    let given = usize::from(selector.is_some()) + usize::from(port.is_some()) + usize::from(device.is_some());
    if given > 1 {
        return Err(PyValueError::new_err("pass at most one of selector, port=, or device="));
    }
    Ok(if let Some(selector) = selector {
        selector.inner.clone()
    } else if let Some(port) = port {
        DeviceSelector::Port(port)
    } else if let Some(device) = device {
        DeviceSelector::Id(device.inner)
    } else {
        DeviceSelector::Only
    })
}

/* === Private Functions === */

/* === Tests === */
