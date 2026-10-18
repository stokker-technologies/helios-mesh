/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `PyO3` adaptation layer mirroring the public HNM Rust facade.

/* === Modules === */

#[macro_use]
mod enumeration;
mod discovery;
mod error;
mod gateway;
mod journal;
mod motion;
mod node;
mod runtime;
mod testing;
mod usb;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::{PyClass, prelude::*};

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type ClassAdder = for<'py> fn(&Bound<'py, PyModule>) -> PyResult<()>;

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn add_class<T: PyClass>(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<T>()
}

fn export_aliases(source: &Bound<'_, PyModule>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    for (name, value) in source.dict().iter() {
        let name = name.extract::<String>()?;
        if name.starts_with('_') {
            continue;
        }
        if let Ok(child) = value.cast::<PyModule>() {
            export_aliases(child, parent)?;
        } else {
            parent.add(&name, value)?;
        }
    }
    Ok(())
}

#[pymodule]
fn _native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    error::register(py, module)?;
    module.add_function(wrap_pyfunction!(discovery::discover, module)?)?;
    module.add_class::<discovery::PyNodeType>()?;
    module.add_class::<discovery::PyDiscoveredNode>()?;
    module.add_class::<discovery::PySidecarNode>()?;
    module.add_class::<discovery::PyRotatorNode>()?;

    module.add_class::<usb::PyUsbDevice>()?;
    module.add_function(wrap_pyfunction!(usb::usb_devices, module)?)?;
    for child in [
        pyo3::wrap_pymodule!(node::node)(py),
        pyo3::wrap_pymodule!(motion::motion)(py),
        pyo3::wrap_pymodule!(gateway::gateway)(py),
        pyo3::wrap_pymodule!(journal::journal)(py),
    ] {
        let child = child.bind(py);
        module.add_submodule(child)?;
        export_aliases(child, module)?;
    }
    testing::register(py, module)
}

/* === Tests === */
