/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Board-owned auxiliary permissions are checked before touching GPIOs.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::mem::ManuallyDrop;

// Third Party Imports
use embassy_rp::gpio::Flex;

// Local Imports
use crate::{
    auxiliary::Pin,
    gpio,
    hardware::{Drive, Pull},
    restart,
};
use helios_mesh_core::platform::led;
use helios_mesh_protocol::node::{AUX_GPIO_COUNT, AuxGpioAccess, AuxGpioMode, Bootloader};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Platform {
    auxiliary: &'static gpio::State,
    restart: &'static restart::Requests,
    led: led::Handle,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Platform {
    pub(crate) fn new(auxiliary: &'static gpio::State, restart: &'static restart::Requests, led: led::Handle) -> Self {
        Self { auxiliary, restart, led }
    }
}

impl helios_mesh_core::platform::Platform for Platform {
    fn auxiliary_gpio_modes(&self) -> [AuxGpioMode; AUX_GPIO_COUNT] {
        self.auxiliary.with(|gpio_state| gpio_state.modes())
    }
    fn auxiliary_gpio_access(&self) -> [AuxGpioAccess; AUX_GPIO_COUNT] {
        self.auxiliary.with(|gpio_state| gpio_state.access())
    }
    fn restart(&mut self, bootloader: Option<Bootloader>) {
        self.restart.schedule(bootloader);
    }
    fn led(&self) -> led::Handle {
        self.led
    }
}

impl Pin for ManuallyDrop<Flex<'_>> {
    fn configure(&mut self, drive: Drive, pull: Pull) {
        self.set_drive_strength(match drive {
            Drive::Ma2 => embassy_rp::gpio::Drive::_2mA,
            Drive::Ma4 => embassy_rp::gpio::Drive::_4mA,
            Drive::Ma8 => embassy_rp::gpio::Drive::_8mA,
            Drive::Ma12 => embassy_rp::gpio::Drive::_12mA,
        });
        self.set_pull(match pull {
            Pull::None => embassy_rp::gpio::Pull::None,
            Pull::Up => embassy_rp::gpio::Pull::Up,
            Pull::Down => embassy_rp::gpio::Pull::Down,
        });
    }
    fn set_mode(&mut self, mode: AuxGpioMode) {
        let pin = self;
        match mode {
            AuxGpioMode::HighImpedance => pin.set_as_input(),
            AuxGpioMode::DriveHigh => {
                pin.set_high();
                pin.set_as_output();
            }
            AuxGpioMode::DriveLow => {
                pin.set_low();
                pin.set_as_output();
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
