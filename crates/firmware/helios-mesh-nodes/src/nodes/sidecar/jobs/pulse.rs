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
use helios_mesh::{Job, JobContext, JobError, RequestError, AuxiliaryPin, AuxGpioMode, Duration};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Pulse { duration_ms: u32 }

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Pulse {
        pub fn new(duration_ms: u32) -> Result<Self, RequestError> {
        if !(1..=60_000).contains(&duration_ms) { return Err(RequestError::InvalidInput); }
        Ok(Self { duration_ms })
    }
}

impl Job for Pulse {
    type Resources = AuxiliaryPin;

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_millis(u64::from(self.duration_ms) + 1_000))
    }

    async fn run(&mut self, pin: &mut AuxiliaryPin, cx: &mut JobContext) -> Result<(), RequestError> {
        pin.set(AuxGpioMode::DriveHigh)?;
        cx.sleep(Duration::from_millis(u64::from(self.duration_ms))).await;
        Ok(())
    }

    fn cleanup(&mut self, pin: &mut AuxiliaryPin, _outcome: &Result<(), JobError>) -> impl core::future::Future<Output = Result<(), RequestError>> {
        core::future::poll_fn(|_| core::task::Poll::Ready(pin.set(AuxGpioMode::DriveLow)))
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
