/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Motion module registration and wire submodule composition.

/* === Modules === */

mod axes;
mod axis;
mod values;
pub(crate) mod wire;

/* === Re-Exports === */

pub(crate) use axes::PyAxesGuard;
pub(crate) use axis::PyAxis;
pub(crate) use values::{PyDelta, PyDescription, PyKind, PyStepperConfig, PyTarget, PyTiming};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{ClassAdder, add_class};

/* === Constants === */

const CLASSES: &[ClassAdder] = &[
    add_class::<PyKind>,
    add_class::<PyTarget>,
    add_class::<PyDelta>,
    add_class::<PyDescription>,
    add_class::<PyTiming>,
    add_class::<PyStepperConfig>,
    add_class::<PyAxis>,
    add_class::<PyAxesGuard>,
];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[pymodule]
pub(crate) fn motion(module: &Bound<'_, PyModule>) -> PyResult<()> {
    for add in CLASSES {
        add(module)?;
    }
    module.add_submodule(pyo3::wrap_pymodule!(wire::wire)(module.py()).bind(module.py()))
}

/* === Private Functions === */

/* === Tests === */
