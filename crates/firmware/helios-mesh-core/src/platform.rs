/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Board services consumed by the chassis, passed by value into composition.

/* === Modules === */

pub mod led;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh_protocol::node::{AUX_GPIO_COUNT, AuxGpioAccess, AuxGpioMode, Bootloader};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

pub trait Platform {
    fn auxiliary_gpio_access(&self) -> [AuxGpioAccess; AUX_GPIO_COUNT];

    fn auxiliary_gpio_modes(&self) -> [AuxGpioMode; AUX_GPIO_COUNT];

    fn restart(&mut self, bootloader: Option<Bootloader>);

    fn led(&self) -> led::Handle;

    fn identify(&mut self) -> bool {
        self.led().identify()
    }
}

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
