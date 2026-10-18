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
use embassy_time::Duration;

// Local Imports
use helios_mesh::{Job, JobContext, JobError, RequestError as Error};
use super::super::MotorChannel;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Stop { pub(crate) active: bool }

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Job for Stop {
    type Resources = MotorChannel;
    async fn run(&mut self, resources: &mut MotorChannel, cx: &mut JobContext) -> Result<(), Error> {
        resources.motor.try_stop(cx.operation()).map_err(|_| Error::Busy)?;
        self.active = true;
        resources.completion(cx.operation(), Duration::from_secs(30)).await?;
        self.active = false;
        Ok(())
    }
    async fn cleanup(&mut self, resources: &mut MotorChannel, _outcome: &Result<(), JobError>) -> Result<(), Error> {
        if self.active {
            resources.settle_after_cancel().await?;
            self.active = false;
        }
        Ok(())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
