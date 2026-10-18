/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Implement boot for firmware helios_mesh core tests runtime.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_futures::block_on;

// Local Imports
use helios_mesh_core::boot::{Boot, CatalogError, Outcome, Satellite};
use helios_mesh_protocol::satellite::{SatelliteName, SatelliteState};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct Radio;

struct Display;

struct MotorWing;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Satellite for Radio {
    const NAME: SatelliteName = SatelliteName::Radio;

    type Ready = u8;
    type Fault = ();

    fn bring_up(self) -> impl core::future::Future<Output = Outcome<Self::Ready, Self::Fault>> {
        core::future::ready(Outcome::Ready(42))
    }
}

impl Satellite for Display {
    const NAME: SatelliteName = SatelliteName::DisplayTft;

    type Ready = ();
    type Fault = ();

    fn bring_up(self) -> impl core::future::Future<Output = Outcome<Self::Ready, Self::Fault>> {
        core::future::ready(Outcome::Absent)
    }
}

impl Satellite for MotorWing {
    const NAME: SatelliteName = SatelliteName::MotorWing;

    type Ready = ();
    type Fault = u8;

    fn bring_up(self) -> impl core::future::Future<Output = Outcome<Self::Ready, Self::Fault>> {
        core::future::ready(Outcome::Faulted(7))
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[test]
fn outcomes_seal_sequence() {
    let mut boot = Boot::new();

    assert_eq!(block_on(boot.bring_up(Radio)).unwrap(), Some(42));
    assert_eq!(block_on(boot.bring_up(Display)).unwrap(), None);
    assert_eq!(block_on(boot.bring_up(MotorWing)).unwrap(), None);

    let report = boot.seal();
    assert_eq!(report.state(SatelliteName::Radio), Some(SatelliteState::Ready));
    assert_eq!(report.state(SatelliteName::DisplayTft), Some(SatelliteState::Absent));
    assert_eq!(report.state(SatelliteName::MotorWing), Some(SatelliteState::Faulted));
    assert_eq!(report.state(SatelliteName::DisplayOled), None);
    assert!(report.is_ready(SatelliteName::Radio));
    assert!(!report.is_ready(SatelliteName::MotorWing));
}

#[test]
fn duplicate_catalog_rejected() {
    let mut boot = Boot::new();

    assert_eq!(block_on(boot.bring_up(Radio)).unwrap(), Some(42));
    assert!(matches!(block_on(boot.bring_up(Radio)), Err(helios_mesh_core::Error::Catalog(CatalogError::Duplicate(SatelliteName::Radio)))));
    assert_eq!(boot.seal().satellites().len(), 1);
}
