/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Universal node module registration.

/* === Modules === */

mod description;
mod diagnostics;
mod enums;
mod handles;
mod identity;
mod journal;
mod selection;
mod status;

/* === Re-Exports === */

pub(crate) use description::{PyFirmwareVersion, PyNodeDescription, PyNodeInfo};
pub(crate) use diagnostics::{PyJournalSummary, PyNodeDiagnostics, PyRequestDiagnostics};
pub(crate) use enums::{PyAuxGpioAccess, PyAuxGpioError, PyAuxGpioMode, PyBootloader, PyOperationState, PySatelliteName, PySatelliteState};
pub(crate) use handles::{PyNodeHandle, PyOperation};
pub(crate) use identity::{PyAuxGpioId, PyChannelId, PyNodeId, PyOperationId};
pub(crate) use journal::{PyJournalEntry, PyJournalEvent, PyJournalPage};
pub(crate) use selection::PyDeviceSelector;
pub(crate) use status::{PyNodeStatus, PyOperationSnapshot, PySatelliteReport};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{ClassAdder, add_class};
use hnm::node::{AUX_GPIO_COUNT, JOURNAL_PAGE_SIZE, MAX_CHANNELS, MAX_SATELLITES};

/* === Constants === */

const CLASSES: &[ClassAdder] = &[
    add_class::<PyDeviceSelector>,
    add_class::<PyNodeId>,
    add_class::<PyChannelId>,
    add_class::<PyOperationId>,
    add_class::<PyAuxGpioAccess>,
    add_class::<PyAuxGpioError>,
    add_class::<PyAuxGpioId>,
    add_class::<PyAuxGpioMode>,
    add_class::<PyBootloader>,
    add_class::<PyOperationState>,
    add_class::<PySatelliteName>,
    add_class::<PySatelliteState>,
    add_class::<PyFirmwareVersion>,
    add_class::<PyNodeInfo>,
    add_class::<PyNodeDescription>,
    add_class::<PySatelliteReport>,
    add_class::<PyNodeStatus>,
    add_class::<PyOperationSnapshot>,
    add_class::<PyRequestDiagnostics>,
    add_class::<PyJournalSummary>,
    add_class::<PyNodeDiagnostics>,
    add_class::<PyJournalEvent>,
    add_class::<PyJournalEntry>,
    add_class::<PyJournalPage>,
    add_class::<PyNodeHandle>,
    add_class::<PyOperation>,
];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[pymodule]
pub(crate) fn node(module: &Bound<'_, PyModule>) -> PyResult<()> {
    for add in CLASSES {
        add(module)?;
    }
    module.add("AUX_GPIO_COUNT", AUX_GPIO_COUNT)?;
    module.add("JOURNAL_PAGE_SIZE", JOURNAL_PAGE_SIZE)?;
    module.add("MAX_CHANNELS", MAX_CHANNELS)?;
    module.add("MAX_SATELLITES", MAX_SATELLITES)
}

/* === Private Functions === */

/* === Tests === */
