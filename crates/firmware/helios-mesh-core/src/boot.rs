/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// One-shot boot-time satellite bring-up and its sealed public report.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::future::Future;

// Third Party Imports
use defmt::Format;

// Local Imports
use helios_mesh_protocol::satellite::{MAX_SATELLITES, SatelliteName, SatelliteReport, SatelliteState};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Boot {
    reports: heapless::Vec<SatelliteReport, MAX_SATELLITES>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BootReport {
    reports: heapless::Vec<SatelliteReport, MAX_SATELLITES>,
}

/* === Enums === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome<T, F = ()> {
    Ready(T),

    Absent,

    Faulted(F),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CatalogError {
    #[error("satellite {0:?} is registered twice")]
    Duplicate(SatelliteName),

    #[error("the satellite catalog is full")]
    Full,
}

/* === Traits === */

pub trait Satellite: Sized {
    const NAME: SatelliteName;

    type Ready;

    type Fault: Format;

    fn bring_up(self) -> impl Future<Output = Outcome<Self::Ready, Self::Fault>>;
}

/* === Implementations === */

impl Boot {
    #[must_use]
    pub const fn new() -> Self {
        Self { reports: heapless::Vec::new() }
    }

    pub async fn bring_up<S: Satellite>(&mut self, satellite: S) -> crate::Result<Option<S::Ready>> {
        self.reserve(S::NAME)?;
        let (state, ready) = match satellite.bring_up().await {
            Outcome::Ready(ready) => (SatelliteState::Ready, Some(ready)),
            Outcome::Absent => (SatelliteState::Absent, None),
            Outcome::Faulted(fault) => {
                warn_fault(S::NAME, &fault);
                (SatelliteState::Faulted, None)
            }
        };
        self.reports.push(SatelliteReport { name: S::NAME, state }).map_err(|_| CatalogError::Full)?;
        Ok(ready)
    }

    #[must_use]
    pub fn seal(self) -> BootReport {
        BootReport { reports: self.reports }
    }

    fn reserve(&self, name: SatelliteName) -> Result<(), CatalogError> {
        if self.reports.iter().any(|report| report.name == name) {
            return Err(CatalogError::Duplicate(name));
        }
        if self.reports.is_full() {
            return Err(CatalogError::Full);
        }
        Ok(())
    }
}

impl BootReport {
    #[must_use]
    pub fn satellites(&self) -> &[SatelliteReport] {
        &self.reports
    }

    #[must_use]
    pub fn state(&self, name: SatelliteName) -> Option<SatelliteState> {
        self.reports.iter().find(|report| report.name == name).map(|report| report.state)
    }

    #[must_use]
    pub fn is_ready(&self, name: SatelliteName) -> bool {
        self.state(name) == Some(SatelliteState::Ready)
    }

    pub(crate) fn wire_reports(&self) -> heapless::Vec<SatelliteReport, MAX_SATELLITES> {
        self.reports.clone()
    }
}

impl Format for CatalogError {
    fn format(&self, formatter: defmt::Formatter) {
        match self {
            Self::Duplicate(SatelliteName::Radio) => {
                defmt::write!(formatter, "duplicate radio satellite");
            }
            Self::Duplicate(SatelliteName::DisplayTft) => {
                defmt::write!(formatter, "duplicate TFT display satellite");
            }
            Self::Duplicate(SatelliteName::DisplayOled) => {
                defmt::write!(formatter, "duplicate OLED display satellite");
            }
            Self::Duplicate(SatelliteName::MotorWing) => {
                defmt::write!(formatter, "duplicate motor wing satellite");
            }
            Self::Full => {
                defmt::write!(formatter, "satellite catalog full");
            }
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn warn_fault<F: Format>(name: SatelliteName, fault: &F) {
    match name {
        SatelliteName::Radio => defmt::warn!("radio satellite bring-up faulted: {}", fault),
        SatelliteName::DisplayTft => {
            defmt::warn!("TFT display satellite bring-up faulted: {}", fault);
        }
        SatelliteName::DisplayOled => {
            defmt::warn!("OLED display satellite bring-up faulted: {}", fault);
        }
        SatelliteName::MotorWing => {
            defmt::warn!("motor wing satellite bring-up faulted: {}", fault);
        }
    }
}

/* === Tests === */
