/* ***
 *
 * Helios Node Mesh
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
use hnm::{Job, JobContext, JobError, RequestError as Error};
use super::super::MotorChannel;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Home { pub(crate) active: bool }

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Job for Home {
    type Resources = MotorChannel;
    fn initialize(&self, resources: &MotorChannel) -> Result<(), Error> { if resources.faulted { return Err(Error::Unavailable); } resources.configuration.ok_or(Error::Conflict).map(|_| ()) }
    async fn run(&mut self, resources: &mut MotorChannel, cx: &mut JobContext) -> Result<(), Error> {
        resources.motor.try_set_home(cx.operation()).map_err(|_| Error::Busy)?;
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
