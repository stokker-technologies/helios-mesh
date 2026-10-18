/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Gateway module registration.

/* === Modules === */

mod bindings;

/* === Re-Exports === */

pub(crate) use bindings::{PyGatewayNode, PyNodePage, PyNodeSummary, resolve_selector};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{ClassAdder, add_class};

/* === Constants === */

const CLASSES: &[ClassAdder] = &[add_class::<PyNodeSummary>, add_class::<PyNodePage>, add_class::<PyGatewayNode>];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[pymodule]
pub(crate) fn gateway(module: &Bound<'_, PyModule>) -> PyResult<()> {
    for add in CLASSES {
        add(module)?;
    }
    module.add("NODE_PAGE_SIZE", helios_mesh::gateway::NODE_PAGE_SIZE)
}

/* === Private Functions === */

/* === Tests === */
