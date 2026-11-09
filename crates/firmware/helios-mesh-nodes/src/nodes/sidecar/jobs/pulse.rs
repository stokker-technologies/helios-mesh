/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Node-Owned Job Workflows

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh::{AuxiliaryPin, Duration, Job, JobContext, JobError, RequestError};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Pulse {
    duration_ms: u32,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Pulse {
    pub fn new(duration_ms: u32) -> Result<Self, RequestError> {
        if !(1..=60_000).contains(&duration_ms) {
            return Err(RequestError::InvalidInput);
        }
        Ok(Self { duration_ms })
    }
}

impl Job for Pulse {
    type Resources = AuxiliaryPin;

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_millis(u64::from(self.duration_ms) + 1_000))
    }

    async fn run(&mut self, pin: &mut AuxiliaryPin, cx: &mut JobContext) -> Result<(), RequestError> {
        let (active, _) = pin.allocation().pulse_modes();
        pin.set(active)?;
        cx.sleep(Duration::from_millis(u64::from(self.duration_ms))).await;
        Ok(())
    }

    fn cleanup(
        &mut self, pin: &mut AuxiliaryPin, _outcome: &Result<(), JobError>,
    ) -> impl core::future::Future<Output = Result<(), RequestError>> {
        let (_, idle) = pin.allocation().pulse_modes();
        core::future::poll_fn(move |_| core::task::Poll::Ready(pin.set(idle)))
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
