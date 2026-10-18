/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Explicit hardware selection and the shared display residents.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_executor::Spawner;

// Local Imports
use crate::{
    satellites::{DisplayOled, DisplayTft},
    ui,
};
use helios_mesh_board_feather_rp2040 as board;
use helios_mesh_core::{boot::Boot, ui as core_ui};
use helios_mesh_drivers::display;

/* === Constants === */

// Each firmware binary has one display projection, independent of its role.
static UI: display::SnapshotWatch = display::SnapshotWatch::new();

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

pub enum Display {
    Tft(board::TftResources),

    Oled(board::OledResources),
}

/* === Traits === */

/* === Implementations === */

impl Display {
    pub async fn bring_up(board: &mut board::Board, boot: &mut Boot) -> crate::Result<Option<Self>> {
        if let Some(tft) = board.take_tft() {
            return Ok(boot.bring_up(DisplayTft(tft)).await?.map(Self::Tft));
        }
        if let Some(oled) = board.take_oled() {
            return Ok(boot.bring_up(DisplayOled(oled)).await?.map(Self::Oled));
        }
        Ok(None)
    }

    pub fn spawn(self, spawner: Spawner, snapshots: core_ui::Receiver) {
        let Some(pages) = UI.receiver() else {
            return;
        };
        match self {
            Self::Tft(tft) => spawner.spawn(defmt::unwrap!(tft_task(tft.into_panel(), pages))),
            Self::Oled(oled) => spawner.spawn(defmt::unwrap!(oled_task(oled.into_panel(), pages))),
        }
        spawner.spawn(defmt::unwrap!(ui_task(snapshots)));
    }
}

/* === Public Functions === */

#[must_use]
pub fn hardware(wing: board::Wing) -> board::Hardware {
    // Unknown mode intentionally suppresses every header capability. The
    // validator also rejects attempts to add a wing or GPIO to that mode.
    if cfg!(feature = "hardware-unknown") {
        return board::Hardware::unknown();
    }
    board::Hardware::new().wing(wing)
}

/* === Private Functions === */

#[embassy_executor::task]
async fn ui_task(snapshots: core_ui::Receiver) -> ! {
    ui::run(snapshots, &UI).await
}

#[embassy_executor::task]
async fn oled_task(panel: board::OledPanel, pages: display::SnapshotReceiver) -> ! {
    panel.run(pages).await
}

// Embassy allocates the band's framebuffer in the task's static future.
#[allow(clippy::large_futures)]
#[embassy_executor::task]
async fn tft_task(panel: board::TftPanel, pages: display::SnapshotReceiver) -> ! {
    panel.run(pages).await
}

/* === Tests === */
