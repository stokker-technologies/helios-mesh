/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Journal module registration.

/* === Modules === */

mod bindings;

/* === Re-Exports === */

pub(crate) use bindings::{PyCursor, PyHeader, PyMetadata, PyRecord};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{ClassAdder, add_class};
use bindings::{event_name, now_us};

/* === Constants === */

const CLASSES: &[ClassAdder] = &[add_class::<PyHeader>, add_class::<PyMetadata>, add_class::<PyRecord>, add_class::<PyCursor>];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[pymodule]
pub(crate) fn journal(module: &Bound<'_, PyModule>) -> PyResult<()> {
    for add in CLASSES {
        add(module)?;
    }
    module.add("SCHEMA_VERSION", hnm::journal::SCHEMA_VERSION)?;
    module.add("RECORD_VERSION", hnm::journal::RECORD_VERSION)?;
    module.add_function(wrap_pyfunction!(event_name, module)?)?;
    module.add_function(wrap_pyfunction!(now_us, module)?)
}

/* === Private Functions === */

/* === Tests === */
