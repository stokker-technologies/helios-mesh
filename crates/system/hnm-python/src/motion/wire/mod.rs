/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Protocol-shaped motion configuration module.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use pyo3::prelude::*;

// Local Imports
use crate::{ClassAdder, add_class};
use hnm::motion::wire as hnm_wire;

/* === Constants === */

const CLASSES: &[ClassAdder] = &[
    add_class::<PyMicrosteps>,
    add_class::<PySettings>,
    add_class::<PyLimits>,
    add_class::<PyMechanics>,
    add_class::<PyPower>,
    add_class::<PyStepperConfiguration>,
    add_class::<PyConfiguration>,
];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[pyclass(name = "Microsteps", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyMicrosteps {
    inner: hnm_wire::Microsteps,
}

#[pyclass(name = "Settings", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PySettings {
    inner: hnm_wire::Settings,
}

#[pyclass(name = "Limits", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyLimits {
    inner: hnm_wire::Limits,
}

#[pyclass(name = "Mechanics", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyMechanics {
    inner: hnm_wire::Mechanics,
}

#[pyclass(name = "Power", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyPower {
    inner: hnm_wire::Power,
}

#[pyclass(name = "StepperConfiguration", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyStepperConfiguration {
    inner: hnm_wire::StepperConfiguration,
}

#[pyclass(name = "Configuration", module = "hnm.motion.wire", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct PyConfiguration {
    inner: hnm_wire::Configuration,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl PyMicrosteps {
    pub(crate) const fn wrap(inner: hnm_wire::Microsteps) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMicrosteps {
    #[classattr]
    const FOUR: Self = Self::wrap(hnm_wire::Microsteps::Four);
    #[classattr]
    const EIGHT: Self = Self::wrap(hnm_wire::Microsteps::Eight);

    #[getter]
    const fn name(&self) -> &'static str {
        match self.inner {
            hnm_wire::Microsteps::Four => "four",
            hnm_wire::Microsteps::Eight => "eight",
        }
    }

    #[getter]
    const fn count(&self) -> u32 {
        self.inner.count()
    }

    const fn __str__(&self) -> &'static str {
        self.name()
    }

    fn __repr__(&self) -> String {
        format!("Microsteps.{}", self.name().to_ascii_uppercase())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PySettings {
    #[new]
    #[pyo3(signature = (*, speed = None, acceleration = None))]
    const fn new(speed: Option<u16>, acceleration: Option<u16>) -> Self {
        Self { inner: hnm_wire::Settings { speed, acceleration } }
    }

    #[getter]
    const fn speed(&self) -> Option<u16> {
        self.inner.speed
    }

    #[getter]
    const fn acceleration(&self) -> Option<u16> {
        self.inner.acceleration
    }

    fn __repr__(&self) -> String {
        format!("Settings(speed={:?}, acceleration={:?})", self.inner.speed, self.inner.acceleration)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyLimits {
    #[new]
    #[pyo3(signature = (*, speed, acceleration))]
    const fn new(speed: u16, acceleration: u16) -> Self {
        Self { inner: hnm_wire::Limits { speed, acceleration } }
    }

    #[getter]
    const fn speed(&self) -> u16 {
        self.inner.speed
    }

    #[getter]
    const fn acceleration(&self) -> u16 {
        self.inner.acceleration
    }

    fn __repr__(&self) -> String {
        format!("Limits(speed={}, acceleration={})", self.inner.speed, self.inner.acceleration)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyMechanics {
    #[new]
    #[pyo3(signature = (full_steps, microsteps, *, reversed = false))]
    const fn new(full_steps: u16, microsteps: &PyMicrosteps, reversed: bool) -> Self {
        Self { inner: hnm_wire::Mechanics { full_steps, microsteps: microsteps.inner, reversed } }
    }

    #[getter]
    const fn full_steps(&self) -> u16 {
        self.inner.full_steps
    }

    #[getter]
    const fn microsteps(&self) -> PyMicrosteps {
        PyMicrosteps::wrap(self.inner.microsteps)
    }

    #[getter]
    const fn reversed(&self) -> bool {
        self.inner.reversed
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyPower {
    #[new]
    const fn new(moving: u8, holding: u8) -> Self {
        Self { inner: hnm_wire::Power { moving, holding } }
    }

    #[getter]
    const fn moving(&self) -> u8 {
        self.inner.moving
    }

    #[getter]
    const fn holding(&self) -> u8 {
        self.inner.holding
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyStepperConfiguration {
    #[new]
    const fn new(mechanics: &PyMechanics, limits: &PyLimits, power: &PyPower) -> Self {
        Self { inner: hnm_wire::StepperConfiguration { mechanics: mechanics.inner, limits: limits.inner, power: power.inner } }
    }

    #[getter]
    const fn mechanics(&self) -> PyMechanics {
        PyMechanics { inner: self.inner.mechanics }
    }

    #[getter]
    const fn limits(&self) -> PyLimits {
        PyLimits { inner: self.inner.limits }
    }

    #[getter]
    const fn power(&self) -> PyPower {
        PyPower { inner: self.inner.power }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pymethods]
impl PyConfiguration {
    #[new]
    fn new(py: Python<'_>, steppers: Vec<Py<PyStepperConfiguration>>) -> PyResult<Self> {
        let mut inner = hnm_wire::Configuration { steppers: heapless::Vec::new() };
        for stepper in steppers {
            inner
                .steppers
                .push(stepper.borrow(py).inner)
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("a configuration holds at most four stepper definitions"))?;
        }
        Ok(Self { inner })
    }

    #[getter]
    fn steppers(&self) -> Vec<PyStepperConfiguration> {
        self.inner.steppers.iter().map(|stepper| PyStepperConfiguration { inner: *stepper }).collect()
    }

    fn __repr__(&self) -> String {
        format!("Configuration(steppers={})", self.inner.steppers.len())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/* === Public Functions === */

#[pymodule]
pub(crate) fn wire(module: &Bound<'_, PyModule>) -> PyResult<()> {
    for add in CLASSES {
        add(module)?;
    }
    Ok(())
}

/* === Private Functions === */

/* === Tests === */
