/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python node enums bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use hnm::node::{AuxGpioAccess, AuxGpioError, AuxGpioMode, Bootloader, OperationState, SatelliteName, SatelliteState};

/* === Constants === */

/* === Macros === */

unit_enum!(
    PyAuxGpioAccess, AuxGpioAccess, "AuxGpioAccess", "hnm.node" {
        NotAllocated => NOT_ALLOCATED, "not_allocated";
        Reserved => RESERVED, "reserved";
        InputOnly => INPUT_ONLY, "input_only";
        PushPull => PUSH_PULL, "push_pull";
    }
);

unit_enum!(
    PyAuxGpioError, AuxGpioError, "AuxGpioError", "hnm.node" {
        InvalidId => INVALID_ID, "invalid_id";
        NotAllocated => NOT_ALLOCATED, "not_allocated";
        Reserved => RESERVED, "reserved";
        ModeNotAllowed => MODE_NOT_ALLOWED, "mode_not_allowed";
        InvalidDuration => INVALID_DURATION, "invalid_duration";
        Busy => BUSY, "busy";
    }
);

unit_enum!(
    PyAuxGpioMode, AuxGpioMode, "AuxGpioMode", "hnm.node" {
        HighImpedance => HIGH_IMPEDANCE, "high_impedance";
        DriveHigh => DRIVE_HIGH, "drive_high";
        DriveLow => DRIVE_LOW, "drive_low";
    }
);

unit_enum!(
    PyBootloader, Bootloader, "Bootloader", "hnm.node" {
        Usb => USB, "usb";
    }
);

unit_enum!(
    PyOperationState, OperationState, "OperationState", "hnm.node" {
        Active => ACTIVE, "active";
        Completed => COMPLETED, "completed";
        Stopped => STOPPED, "stopped";
        Failed => FAILED, "failed";
    }
    methods {
                const fn is_terminal(&self) -> bool {
            !matches!(self.inner, OperationState::Active)
        }
    }
);

unit_enum!(
    PySatelliteName, SatelliteName, "SatelliteName", "hnm.node" {
        Radio => RADIO, "radio";
        DisplayTft => DISPLAY_TFT, "display_tft";
        DisplayOled => DISPLAY_OLED, "display_oled";
        MotorWing => MOTOR_WING, "motor_wing";
    }
);

unit_enum!(
    PySatelliteState, SatelliteState, "SatelliteState", "hnm.node" {
        Ready => READY, "ready";
        Absent => ABSENT, "absent";
        Faulted => FAULTED, "faulted";
    }
);

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "AuxGpioAccess", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAuxGpioAccess {
    pub(crate) inner: AuxGpioAccess,
}

#[pyclass(name = "AuxGpioError", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAuxGpioError {
    pub(crate) inner: AuxGpioError,
}

#[pyclass(name = "AuxGpioMode", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyAuxGpioMode {
    pub(crate) inner: AuxGpioMode,
}

#[pyclass(name = "Bootloader", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyBootloader {
    pub(crate) inner: Bootloader,
}

#[pyclass(name = "OperationState", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyOperationState {
    pub(crate) inner: OperationState,
}

#[pyclass(name = "SatelliteName", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySatelliteName {
    pub(crate) inner: SatelliteName,
}

#[pyclass(name = "SatelliteState", module = "hnm.node", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySatelliteState {
    pub(crate) inner: SatelliteState,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
