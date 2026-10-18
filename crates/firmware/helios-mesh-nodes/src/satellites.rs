/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Composition adapters for optional display and motor-wing satellites.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh_board_feather_rp2040 as board;
use helios_mesh_core::boot::{Outcome, Satellite};
use helios_mesh_protocol::satellite::SatelliteName;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct DisplayTft(pub board::TftResources);

pub struct DisplayOled(pub board::OledResources);

pub struct MotorWing(pub board::MotorResources);

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Satellite for DisplayTft {
    const NAME: SatelliteName = SatelliteName::DisplayTft;
    type Ready = board::TftResources;
    type Fault = ();

    async fn bring_up(mut self) -> Outcome<Self::Ready, Self::Fault> {
        if self.0.detect().await {
            Outcome::Ready(self.0)
        } else {
            // Dropping the bundle retains the declared electrical idle.
            Outcome::Absent
        }
    }
}

impl Satellite for DisplayOled {
    const NAME: SatelliteName = SatelliteName::DisplayOled;
    type Ready = board::OledResources;
    type Fault = ();

    async fn bring_up(mut self) -> Outcome<Self::Ready, Self::Fault> {
        if self.0.detect().await { Outcome::Ready(self.0) } else { Outcome::Absent }
    }
}

impl Satellite for MotorWing {
    const NAME: SatelliteName = SatelliteName::MotorWing;

    type Ready = board::MotorWing;
    type Fault = ();

    async fn bring_up(self) -> Outcome<Self::Ready, Self::Fault> {
        match self.0.connect().await {
            Ok(wing) => Outcome::Ready(wing),
            // Optional Motor Hardware Retains Its Absent Wire State On Failed Connection.
            Err(error) => {
                defmt::warn!("motor wing unavailable: {:?}", defmt::Debug2Format(&error));
                Outcome::Absent
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
