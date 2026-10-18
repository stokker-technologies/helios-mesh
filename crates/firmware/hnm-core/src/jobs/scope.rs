/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Scoped Child Job Ownership

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::{
    future::poll_fn,
    task::{Context, Poll},
};

// Third Party Imports
use embassy_time::{Duration, Timer};

// Local Imports
use super::{Channel, Job, JobError, JobOperation, JobOutcome, JobStatus};
use hnm_protocol::{error::Error, node::OperationId};

/* === Constants === */

pub const MAX_JOB_CHILDREN: usize = 8;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct JobContext {
    operation: OperationId,
    owner: usize,
    children: heapless::Vec<Child, MAX_JOB_CHILDREN>,
    child_error: Option<Error>,
}

pub(super) struct Child {
    pub control: &'static dyn ChildControl,
    pub id: OperationId,
}

/* === Enums === */

/* === Traits === */

pub(super) trait ChildControl {
    fn cancel(&'static self, id: OperationId);
    fn status(&'static self, id: OperationId) -> JobStatus;
    fn poll_finished(&'static self, id: OperationId, cx: &mut Context<'_>) -> Poll<JobOutcome>;
    fn release(&self, id: OperationId);
}

/* === Implementations === */

impl JobContext {
    pub(super) fn new(operation: OperationId, owner: usize) -> Self {
        Self { operation, owner, children: heapless::Vec::new(), child_error: None }
    }

    #[must_use]
    pub const fn operation(&self) -> OperationId {
        self.operation
    }

    pub async fn sleep(&mut self, duration: Duration) {
        Timer::after(duration).await;
    }

    pub fn submit<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize>(
        &mut self, channel: &Channel<J, R, N, H>, job: impl Into<J>,
    ) -> Result<JobOperation<J, R, N, H>, Error> {
        if channel.scope_key() == self.owner {
            return Err(Error::Conflict);
        }

        // Release finished children so sequential workflows do not exhaust the scope.
        let error = &mut self.child_error;
        self.children.retain(|child| {
            if let JobStatus::Finished(outcome) = child.control.status(child.id) {
                if error.is_none() {
                    *error = child_failure(outcome);
                }
                false
            } else {
                true
            }
        });
        if self.children.is_full() {
            return Err(Error::Busy);
        }

        let operation = channel.submit(job)?;
        // The scope and caller each retain their own observation of the result.
        let retained = channel.operation(operation.id()).expect("new operation has one observer");
        self.children.push(retained.into_child()).unwrap_or_else(|_| unreachable!("scope capacity checked"));
        Ok(operation)
    }

    pub(super) async fn close(&mut self) -> Result<(), Error> {
        for child in &self.children {
            child.control.cancel(child.id);
        }

        poll_fn(|cx| {
            let mut pending = false;
            for child in &self.children {
                match child.control.poll_finished(child.id, cx) {
                    Poll::Ready(outcome) => {
                        if self.child_error.is_none() {
                            self.child_error = child_failure(outcome);
                        }
                    }
                    Poll::Pending => pending = true,
                }
            }
            if pending { Poll::Pending } else { Poll::Ready(()) }
        })
        .await;

        self.children.clear();
        self.child_error.map_or(Ok(()), Err)
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        self.control.release(self.id);
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn child_failure(outcome: JobOutcome) -> Option<Error> {
    // Scope-requested cancellation is normal; failed execution or cleanup is not.
    outcome.cleanup.and_then(Result::err).or(match outcome.result {
        Err(JobError::Rejected(error)) => Some(error),
        Err(JobError::Deadline) => Some(Error::Unavailable),
        _ => None,
    })
}

/* === Tests === */
