/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Bounded jobs with exclusive resources and explicit lifecycle outcomes.

/* === Modules === */

mod channel;
mod scope;

/* === Re-Exports === */

pub use channel::ChannelStorage;
pub use channel::{Channel, ChannelRef, JobOperation, ResourceGuard};
pub use embassy_time::Duration;
pub use scope::{JobContext, MAX_JOB_CHILDREN};

/* === Imports === */

// Standard Library Imports
use core::future::Future;

// Third Party Imports

// Local Imports
use hnm_protocol::{
    error::Error,
    node::{ChannelId, OperationId, OperationSnapshot, OperationState},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobOutcome {
    pub result: Result<(), JobError>,

    pub cleanup: Option<Result<(), Error>>,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum JobError {
    #[error("job rejected: {0}")]
    Rejected(#[from] Error),

    #[error("job cancelled")]
    Cancelled,

    #[error("job deadline expired")]
    Deadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobStatus {
    Queued,

    Running,

    Finished(JobOutcome),
}

/* === Traits === */

pub trait Job {
    type Resources;

    fn timeout(&self) -> Option<Duration> {
        None
    }

    fn initialize(&self, _resources: &Self::Resources) -> Result<(), Error> {
        Ok(())
    }

    fn run(&mut self, resources: &mut Self::Resources, cx: &mut JobContext) -> impl Future<Output = Result<(), Error>>;

    fn cleanup(&mut self, _resources: &mut Self::Resources, _outcome: &Result<(), JobError>) -> impl Future<Output = Result<(), Error>> {
        async { Ok(()) }
    }
}

/* === Implementations === */

impl JobOutcome {
    pub fn into_result(self) -> crate::Result<()> {
        let cleanup = self.cleanup.and_then(core::result::Result::err);
        match (self.result, cleanup) {
            (Err(primary), cleanup) => Err(crate::Error::Job { primary, cleanup }),
            (Ok(()), Some(error)) => Err(crate::Error::Cleanup(error)),
            (Ok(()), None) => Ok(()),
        }
    }
}

impl JobStatus {
    #[must_use]
    pub fn snapshot(self, id: OperationId, channel: ChannelId) -> OperationSnapshot {
        let state = match self {
            Self::Queued | Self::Running => OperationState::Active,
            Self::Finished(JobOutcome { result: Err(JobError::Cancelled), .. }) => OperationState::Stopped,
            Self::Finished(JobOutcome { result: Ok(()), .. }) => OperationState::Completed,
            Self::Finished(_) => OperationState::Failed,
        };

        OperationSnapshot {
            id,
            channel,
            state,
            error: match self {
                Self::Finished(JobOutcome { result: Err(JobError::Rejected(error)), .. }) => Some(error),
                Self::Finished(JobOutcome { result: Err(JobError::Deadline), .. }) => Some(Error::Unavailable),
                _ => None,
            },
            cleanup_error: match self {
                Self::Finished(JobOutcome { cleanup: Some(Err(error)), .. }) => Some(error),
                _ => None,
            },
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
