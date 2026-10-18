/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Unique board pin resources and shared read-only status.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::{cell::RefCell, mem::ManuallyDrop};

// Third Party Imports
use embassy_rp::gpio::Flex;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};

// Local Imports
use crate::auxiliary::AuxiliaryGpio;
use helios_mesh_protocol::{
    error::Error,
    node::{AuxGpioId, AuxGpioMode},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type Pins = AuxiliaryGpio<ManuallyDrop<Flex<'static>>>;

/* === Structs === */

pub(crate) struct State {
    pins: Mutex<CriticalSectionRawMutex, RefCell<Pins>>,
}

pub struct AuxiliaryPin {
    state: &'static State,
    id: AuxGpioId,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl State {
    pub(crate) fn new(pins: Pins) -> Self {
        Self { pins: Mutex::new(RefCell::new(pins)) }
    }
    pub(crate) fn with<T>(&self, action: impl FnOnce(&mut Pins) -> T) -> T {
        self.pins.lock(|pins| {
            let mut pins = pins.borrow_mut();

            action(&mut pins)
        })
    }
    pub(crate) fn take(&'static self, id: AuxGpioId) -> Result<AuxiliaryPin, Error> {
        self.with(|pins| pins.claim(id)).map_err(|reason| Error::AuxGpio { gpio: id, reason })?;
        Ok(AuxiliaryPin { state: self, id })
    }
}

impl AuxiliaryPin {
    pub fn set(&mut self, mode: AuxGpioMode) -> Result<(), Error> {
        self.state.with(|pins| pins.configure(self.id, mode)).map_err(|reason| Error::AuxGpio { gpio: self.id, reason })
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
