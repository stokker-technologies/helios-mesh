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
use embassy_time::{Duration, Instant};

// Local Imports
use helios_mesh::{Job, JobContext, JobError, RequestError as Error};
use helios_mesh::OperationId;
use helios_mesh::motor;
use helios_mesh::motion;
use super::super::{MotorChannel, Requested};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Turn {
    pub(crate) requested: Requested,
    pub(crate) settings: motion::Settings,
    pub(crate) start: Instant,
    pub(crate) active: Option<OperationId>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Job for Turn {
    type Resources = MotorChannel;
    fn initialize(&self, resources: &MotorChannel) -> Result<(), Error> { resources.validate(self.requested, self.settings).map(|_| ()) }
    async fn run(&mut self, resources: &mut MotorChannel, cx: &mut JobContext) -> Result<(), Error> {
        let target = resources.validate(self.requested, self.settings)?;
        let config = resources.configuration.ok_or(Error::Conflict)?;
        let speed = u32::from(self.settings.speed(config.limits)).saturating_mul(1_000_000);
        let acceleration = u32::from(self.settings.acceleration(config.limits)).saturating_mul(1_000_000);
        let distance = (i64::from(target) - i64::from(resources.position)).unsigned_abs();
        let timeout = Duration::from_millis(distance * 2_000 / u64::from(speed) + 30_000);
        let operation = cx.operation();
        resources.motor.try_move(motor::Move { operation, target_microdegrees: target, maximum_microdegrees_per_second: speed,
            acceleration_microdegrees_per_second_squared: acceleration, start: self.start }).map_err(|_| Error::Busy)?;
        self.active = Some(operation);
        resources.completion(operation, timeout).await?;
        self.active = None;
        Ok(())
    }
    async fn cleanup(&mut self, resources: &mut MotorChannel, _outcome: &Result<(), JobError>) -> Result<(), Error> {
        if self.active.take().is_some() {
            resources.settle_after_cancel().await?;
        }
        Ok(())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
