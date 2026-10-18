/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Test-only awaitable seam for exercising [`awaitable`]'s cancellation.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// Third Party Imports
use pyo3::{prelude::*, types::PyModule};

// Local Imports
use crate::runtime::awaitable;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct DropSignal(Arc<AtomicBool>);

#[pyclass(name = "_PendingProbe", module = "helios_mesh._native", frozen)]
pub(crate) struct PyPendingProbe {
    inner: Arc<AtomicBool>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Drop for DropSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[pymethods]
impl PyPendingProbe {
    #[new]
    fn new() -> Self {
        Self { inner: Arc::new(AtomicBool::new(false)) }
    }

    fn wait<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let signal = DropSignal(self.inner.clone());
        awaitable(py, async move {
            let _signal = signal;
            std::future::pending::<helios_mesh::Result<()>>().await
        })
    }

    #[getter]
    fn dropped(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

/* === Public Functions === */

pub(crate) fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPendingProbe>()
}

/* === Private Functions === */

/* === Tests === */
