/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Feather RP2040 restart resident.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_rp::{Peri, peripherals::WATCHDOG, watchdog::Watchdog};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

// Local Imports
use helios_mesh_protocol::node::Bootloader;

/* === Constants === */

const RESPONSE_GRACE_MS: u64 = 100;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub(crate) struct Requests {
    requested: Signal<CriticalSectionRawMutex, Option<Bootloader>>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Requests {
    pub(crate) const fn new() -> Self {
        Self { requested: Signal::new() }
    }

    pub(crate) fn schedule(&self, bootloader: Option<Bootloader>) {
        self.requested.signal(bootloader);
    }
}

/* === Public Functions === */

#[embassy_executor::task]
pub(crate) async fn run(watchdog: Peri<'static, WATCHDOG>, requests: &'static Requests) -> ! {
    let bootloader = requests.requested.wait().await;
    Timer::after_millis(RESPONSE_GRACE_MS).await;

    match bootloader {
        Some(Bootloader::Usb) => embassy_rp::rom_data::reset_to_usb_boot(0, 0),
        None => Watchdog::new(watchdog).trigger_reset(),
    }

    loop {
        core::hint::spin_loop();
    }
}

/* === Private Functions === */

/* === Tests === */
