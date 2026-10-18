/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Python motion values bindings.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::node::PyChannelId;
use helios_mesh::motion::{Delta, Description, Kind, Target, Timing};

/* === Constants === */

/* === Macros === */

unit_enum!(
    PyKind, Kind, "Kind", "helios_mesh.motion" {
        Rotational => ROTATIONAL, "rotational";
        Translational => TRANSLATIONAL, "translational";
    }
);

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "Kind", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyKind {
    pub(crate) inner: Kind,
}

#[pyclass(name = "Target", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyTarget {
    pub(crate) inner: Target,
}

#[pyclass(name = "Delta", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyDelta {
    pub(crate) inner: Delta,
}

#[pyclass(name = "Description", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyDescription {
    inner: Description,
}

#[pyclass(name = "Timing", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyTiming {
    inner: Timing,
}

#[pyclass(name = "StepperConfig", module = "helios_mesh.motion", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyStepperConfig {
    pub(crate) inner: helios_mesh::StepperConfig,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyTarget {
    pub(crate) const fn wrap(inner: Target) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTarget {
    #[staticmethod]
    const fn rotational(microdegrees: i32) -> Self {
        Self::wrap(Target::Rotational { microdegrees })
    }

    #[staticmethod]
    const fn translational(micrometres: i32) -> Self {
        Self::wrap(Target::Translational { micrometres })
    }

    #[getter]
    const fn kind(&self) -> PyKind {
        match self.inner {
            Target::Rotational { .. } => PyKind::wrap(Kind::Rotational),
            Target::Translational { .. } => PyKind::wrap(Kind::Translational),
        }
    }

    #[getter]
    const fn microdegrees(&self) -> Option<i32> {
        match self.inner {
            Target::Rotational { microdegrees } => Some(microdegrees),
            Target::Translational { .. } => None,
        }
    }

    #[getter]
    const fn micrometres(&self) -> Option<i32> {
        match self.inner {
            Target::Rotational { .. } => None,
            Target::Translational { micrometres } => Some(micrometres),
        }
    }

    #[getter]
    fn degrees(&self) -> Option<f64> {
        match self.inner {
            Target::Rotational { microdegrees } => Some(f64::from(microdegrees) / 1_000_000.0),
            Target::Translational { .. } => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            Target::Rotational { microdegrees } => format!("Target.rotational({microdegrees})"),
            Target::Translational { micrometres } => format!("Target.translational({micrometres})"),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PyDelta {
    pub(crate) const fn wrap(inner: Delta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDelta {
    #[staticmethod]
    const fn rotational(microdegrees: i32) -> Self {
        Self::wrap(Delta::Rotational { microdegrees })
    }

    #[staticmethod]
    const fn translational(micrometres: i32) -> Self {
        Self::wrap(Delta::Translational { micrometres })
    }

    #[getter]
    const fn kind(&self) -> PyKind {
        match self.inner {
            Delta::Rotational { .. } => PyKind::wrap(Kind::Rotational),
            Delta::Translational { .. } => PyKind::wrap(Kind::Translational),
        }
    }

    #[getter]
    const fn microdegrees(&self) -> Option<i32> {
        match self.inner {
            Delta::Rotational { microdegrees } => Some(microdegrees),
            Delta::Translational { .. } => None,
        }
    }

    #[getter]
    const fn micrometres(&self) -> Option<i32> {
        match self.inner {
            Delta::Rotational { .. } => None,
            Delta::Translational { micrometres } => Some(micrometres),
        }
    }

    #[getter]
    fn degrees(&self) -> Option<f64> {
        match self.inner {
            Delta::Rotational { microdegrees } => Some(f64::from(microdegrees) / 1_000_000.0),
            Delta::Translational { .. } => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            Delta::Rotational { microdegrees } => format!("Delta.rotational({microdegrees})"),
            Delta::Translational { micrometres } => format!("Delta.translational({micrometres})"),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyDescription {
    #[getter]
    const fn channel(&self) -> PyChannelId {
        PyChannelId::wrap(self.inner.channel)
    }

    #[getter]
    const fn kind(&self) -> PyKind {
        PyKind::wrap(self.inner.kind)
    }

    #[getter]
    const fn minimum(&self) -> i32 {
        self.inner.minimum
    }

    #[getter]
    const fn maximum(&self) -> i32 {
        self.inner.maximum
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PyDescription {
    pub(crate) const fn wrap(inner: Description) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTiming {
    #[getter]
    const fn duration_ms(&self) -> u32 {
        self.inner.duration_ms
    }

    #[getter]
    const fn steps(&self) -> u32 {
        self.inner.steps
    }

    #[getter]
    const fn delayed_steps(&self) -> u32 {
        self.inner.delayed_steps
    }

    #[getter]
    const fn maximum_lateness_us(&self) -> u32 {
        self.inner.maximum_lateness_us
    }

    #[getter]
    const fn delayed_100_to_250_us(&self) -> u32 {
        self.inner.delayed_100_to_250_us
    }

    #[getter]
    const fn delayed_250_to_500_us(&self) -> u32 {
        self.inner.delayed_250_to_500_us
    }

    #[getter]
    const fn delayed_500_to_1000_us(&self) -> u32 {
        self.inner.delayed_500_to_1000_us
    }

    #[getter]
    const fn delayed_over_1000_us(&self) -> u32 {
        self.inner.delayed_over_1000_us
    }

    #[getter]
    const fn maximum_consecutive_delayed_steps(&self) -> u32 {
        self.inner.maximum_consecutive_delayed_steps
    }

    #[getter]
    const fn schedule_rebases(&self) -> u32 {
        self.inner.schedule_rebases
    }

    #[getter]
    const fn maximum_output_queue_us(&self) -> u32 {
        self.inner.maximum_output_queue_us
    }

    #[getter]
    const fn maximum_i2c_update_us(&self) -> u32 {
        self.inner.maximum_i2c_update_us
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PyTiming {
    pub(crate) const fn wrap(inner: Timing) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStepperConfig {
    #[new]
    #[pyo3(signature = (
        *, steps, microsteps, max_speed, max_acceleration, moving_pwm, holding_pwm,
        reversed = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    const fn new(
        steps: u16, microsteps: u8, max_speed: u16, max_acceleration: u16, moving_pwm: u8, holding_pwm: u8, reversed: bool,
    ) -> Self {
        Self { inner: helios_mesh::StepperConfig { steps, microsteps, reversed, max_speed, max_acceleration, moving_pwm, holding_pwm } }
    }

    #[getter]
    const fn steps(&self) -> u16 {
        self.inner.steps
    }

    #[getter]
    const fn microsteps(&self) -> u8 {
        self.inner.microsteps
    }

    #[getter]
    const fn reversed(&self) -> bool {
        self.inner.reversed
    }

    #[getter]
    const fn max_speed(&self) -> u16 {
        self.inner.max_speed
    }

    #[getter]
    const fn max_acceleration(&self) -> u16 {
        self.inner.max_acceleration
    }

    #[getter]
    const fn moving_pwm(&self) -> u8 {
        self.inner.moving_pwm
    }

    #[getter]
    const fn holding_pwm(&self) -> u8 {
        self.inner.holding_pwm
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
