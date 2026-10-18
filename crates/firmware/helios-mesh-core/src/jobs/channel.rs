/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Channel-Owned Jobs And Completion Futures

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::{
    cell::RefCell,
    future::poll_fn,
    ops::{Deref, DerefMut},
    task::Poll,
};

// Third Party Imports
use embassy_futures::select::{Either, select};
use embassy_sync::{
    blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex},
    channel::Channel as Queue,
    mutex::{Mutex, MutexGuard},
    signal::Signal,
    waitqueue::MultiWakerRegistration,
};

// Local Imports
use super::scope::{Child, ChildControl};
use super::{Job, JobContext, JobError, JobOutcome, JobStatus};
use helios_mesh_protocol::{
    error::Error,
    node::{ChannelId, OperationId, OperationSnapshot},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Channel<J: Job<Resources = R> + 'static, R: 'static, const N: usize = 4, const H: usize = 128> {
    storage: &'static ChannelStorage<J, R, N, H>,
}

pub struct ChannelStorage<J: Job<Resources = R>, R, const N: usize = 4, const H: usize = 128> {
    resources: Mutex<CriticalSectionRawMutex, Option<R>>,
    available: bool,
    queue: Queue<CriticalSectionRawMutex, (OperationId, J), N>,
    state: BlockingMutex<CriticalSectionRawMutex, RefCell<State<H>>>,
    cancel: Signal<CriticalSectionRawMutex, ()>,
}

struct State<const H: usize> {
    channel: u8,
    sequence: u32,
    started: bool,
    active: Option<OperationId>,
    records: heapless::Vec<Record, H>,
    waiters: MultiWakerRegistration<16>,
}

struct Record {
    id: OperationId,
    status: JobStatus,
    cancelled: bool,
    observers: usize,
}

#[must_use = "retain to await completion, or read id() for a wire response"]
pub struct JobOperation<J: Job<Resources = R> + 'static, R: 'static, const N: usize = 4, const H: usize = 128> {
    channel: Channel<J, R, N, H>,
    id: OperationId,
}

pub struct ResourceGuard<'a, R>(MutexGuard<'a, CriticalSectionRawMutex, Option<R>>);

#[derive(Clone, Copy)]
pub struct ChannelRef<'a> {
    channel: &'a dyn ChannelControl,
    name: &'static str,
}

/* === Enums === */

/* === Traits === */

trait ChannelControl {
    fn id(&self) -> ChannelId;
    fn is_available(&self) -> bool;
    fn is_idle(&self) -> bool;
    fn queued(&self) -> usize;
    fn status(&self, id: OperationId) -> Option<JobStatus>;
    fn cancel(&self, id: OperationId) -> Result<(), Error>;
    fn cancel_all(&self);
}

/* === Implementations === */

impl<'a> ChannelRef<'a> {
    #[must_use]
    pub fn new<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize>(
        channel: &'a Channel<J, R, N, H>, name: &'static str,
    ) -> Self {
        Self { channel, name }
    }

    #[must_use]
    pub fn id(&self) -> ChannelId {
        self.channel.id()
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.channel.is_available()
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.channel.is_idle()
    }

    #[must_use]
    pub fn queued(&self) -> usize {
        self.channel.queued()
    }

    #[must_use]
    pub fn status(&self, id: OperationId) -> Option<JobStatus> {
        self.channel.status(id)
    }

    #[must_use]
    pub fn snapshot(&self, id: OperationId) -> Option<OperationSnapshot> {
        self.status(id).map(|status| status.snapshot(id, self.id()))
    }

    pub fn cancel(&self, id: OperationId) -> Result<(), Error> {
        self.channel.cancel(id)
    }

    pub fn cancel_all(&self) {
        self.channel.cancel_all();
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> ChannelControl for Channel<J, R, N, H> {
    fn id(&self) -> ChannelId {
        self.storage.state.lock(|state| ChannelId(state.borrow().channel))
    }
    fn is_available(&self) -> bool {
        Channel::is_available(self)
    }
    fn is_idle(&self) -> bool {
        Channel::is_idle(self)
    }
    fn queued(&self) -> usize {
        self.storage.queue.len()
    }
    fn status(&self, id: OperationId) -> Option<JobStatus> {
        Channel::status(self, id)
    }
    fn cancel(&self, id: OperationId) -> Result<(), Error> {
        Channel::cancel(self, id)
    }
    fn cancel_all(&self) {
        Channel::cancel_all(self);
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> Copy for Channel<J, R, N, H> {}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> Clone for Channel<J, R, N, H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Deref for ResourceGuard<'_, R> {
    type Target = R;
    fn deref(&self) -> &R {
        self.0.as_ref().expect("available channel")
    }
}

impl<R> DerefMut for ResourceGuard<'_, R> {
    fn deref_mut(&mut self) -> &mut R {
        self.0.as_mut().expect("available channel")
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> Channel<J, R, N, H> {
    pub(super) fn scope_key(&self) -> usize {
        core::ptr::from_ref(self.storage) as usize
    }

    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.storage.available
    }

    pub fn try_resources(&self) -> Result<ResourceGuard<'_, R>, Error> {
        if !self.is_available() {
            return Err(Error::Unavailable);
        }
        self.storage.resources.try_lock().map(ResourceGuard).map_err(|_| Error::Busy)
    }

    pub fn can_submit(&self) -> Result<(), Error> {
        if !self.is_available() {
            return Err(Error::Unavailable);
        }
        self.storage.state.lock(|state| self.storage.check_admission(&state.borrow()))
    }

    pub fn submit(&self, job: impl Into<J>) -> Result<JobOperation<J, R, N, H>, Error> {
        let job = job.into();
        if !self.is_available() {
            return Err(Error::Unavailable);
        }
        self.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            self.storage.check_admission(&state)?;
            if state.records.is_full() {
                let old = state.records.iter().position(Record::evictable).expect("admission checked");
                state.records.remove(old);
            }
            state.sequence += 1;
            let id = OperationId((state.sequence << 8) | u32::from(state.channel));
            self.storage.queue.try_send((id, job)).map_err(|_| Error::Busy)?;
            state
                .records
                .push(Record { id, status: JobStatus::Queued, cancelled: false, observers: 1 })
                .unwrap_or_else(|_| unreachable!("admission reserved history space"));
            Ok(JobOperation { channel: *self, id })
        })
    }

    pub fn operation(&self, id: OperationId) -> Result<JobOperation<J, R, N, H>, Error> {
        self.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            let record = state.records.iter_mut().find(|record| record.id == id).ok_or(Error::NotFound)?;
            record.observers = record.observers.checked_add(1).ok_or(Error::Busy)?;
            Ok(JobOperation { channel: *self, id })
        })
    }

    #[must_use]
    pub fn status(&self, id: OperationId) -> Option<JobStatus> {
        self.storage.state.lock(|state| state.borrow().records.iter().find(|record| record.id == id).map(|record| record.status))
    }

    #[must_use]
    pub fn snapshot(&self, id: OperationId) -> Option<OperationSnapshot> {
        let channel = self.storage.state.lock(|state| ChannelId(state.borrow().channel));
        self.status(id).map(|status| status.snapshot(id, channel))
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.storage.state.lock(|state| state.borrow().records.iter().all(|record| matches!(record.status, JobStatus::Finished(_))))
    }

    pub fn cancel(&self, id: OperationId) -> Result<(), Error> {
        self.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            let record = state.records.iter_mut().find(|record| record.id == id).ok_or(Error::NotFound)?;
            if matches!(record.status, JobStatus::Finished(_)) {
                return Ok(());
            }
            record.cancelled = true;
            if state.active == Some(id) {
                self.storage.cancel.signal(());
            }
            Ok(())
        })
    }

    pub fn cancel_all(&self) {
        let (discarded, mut waiters) = self.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            let mut discarded = heapless::Vec::<J, N>::new();
            for record in &mut state.records {
                if !matches!(record.status, JobStatus::Finished(_)) {
                    record.cancelled = true;
                }
            }
            while let Ok((id, job)) = self.storage.queue.try_receive() {
                let record = state.records.iter_mut().find(|record| record.id == id).expect("queued record retained");
                record.status = JobStatus::Finished(cancelled_outcome());
                discarded.push(job).unwrap_or_else(|_| unreachable!("drained queue fits its capacity"));
            }
            if state.active.is_some() {
                self.storage.cancel.signal(());
            }
            let waiters = core::mem::replace(&mut state.waiters, MultiWakerRegistration::new());
            (discarded, waiters)
        });
        // User job destructors may themselves release operation handles.
        drop(discarded);
        waiters.wake();
    }

    pub async fn run(self) -> ! {
        self.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            assert!(!state.started, "channel worker already started");
            state.started = true;
        });
        loop {
            self.storage.run_next().await;
        }
    }
}

impl Record {
    fn evictable(&self) -> bool {
        self.observers == 0 && matches!(self.status, JobStatus::Finished(_))
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> JobOperation<J, R, N, H> {
    pub(super) fn into_child(self) -> Child {
        let child = Child { control: self.channel.storage, id: self.id };
        core::mem::forget(self); // Transfer this observer's ownership to Child::drop.
        child
    }

    #[must_use]
    pub const fn id(&self) -> OperationId {
        self.id
    }

    pub fn cancel(&self) -> Result<(), Error> {
        self.channel.cancel(self.id)
    }

    pub async fn finished(&self) -> JobOutcome {
        poll_fn(|cx| {
            self.channel.storage.state.lock(|state| {
                let mut state = state.borrow_mut();
                let record = state.records.iter().find(|record| record.id == self.id).expect("observer retains history");
                if let JobStatus::Finished(outcome) = record.status {
                    return Poll::Ready(outcome);
                }
                state.waiters.register(cx.waker());
                Poll::Pending
            })
        })
        .await
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> Drop for JobOperation<J, R, N, H> {
    fn drop(&mut self) {
        self.channel.storage.state.lock(|state| {
            let mut state = state.borrow_mut();
            let record = state.records.iter_mut().find(|record| record.id == self.id).expect("observer retains history");
            record.observers -= 1;
        });
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> ChannelStorage<J, R, N, H> {
    pub fn new(channel: u8, resources: Option<R>) -> Self {
        assert!(channel != 0 && N > 0 && H > N);
        Self {
            available: resources.is_some(),
            resources: Mutex::new(resources),
            queue: Queue::new(),
            state: BlockingMutex::new(RefCell::new(State {
                channel,
                sequence: 0,
                started: false,
                active: None,
                records: heapless::Vec::new(),
                waiters: MultiWakerRegistration::new(),
            })),
            cancel: Signal::new(),
        }
    }

    #[must_use]
    pub fn channel(&'static self) -> Channel<J, R, N, H> {
        Channel { storage: self }
    }

    fn check_admission(&self, state: &State<H>) -> Result<(), Error> {
        if state.sequence == 0x00ff_ffff {
            return Err(Error::Unavailable);
        }
        if self.queue.is_full() || (state.records.is_full() && !state.records.iter().any(Record::evictable)) {
            return Err(Error::Busy);
        }
        Ok(())
    }

    fn update(&self, id: OperationId, outcome: JobOutcome) {
        let mut waiters = self.state.lock(|state| {
            let mut state = state.borrow_mut();
            state.active = None;
            let record = state.records.iter_mut().find(|record| record.id == id).expect("admitted job retained");
            record.status = JobStatus::Finished(outcome);
            core::mem::replace(&mut state.waiters, MultiWakerRegistration::new())
        });
        waiters.wake();
    }

    async fn run_next(&self) {
        let (id, mut job) = self.queue.receive().await;
        let cancelled = self.state.lock(|state| {
            let mut state = state.borrow_mut();
            self.cancel.reset();
            state.active = Some(id);
            let record = state.records.iter_mut().find(|record| record.id == id).expect("admitted job retained");
            record.status = JobStatus::Running;
            record.cancelled
        });
        if cancelled {
            self.update(id, cancelled_outcome());
            return;
        }

        let mut resources = match select(self.resources.lock(), self.cancel.wait()).await {
            Either::First(resources) => resources,
            Either::Second(()) => {
                self.update(id, cancelled_outcome());
                return;
            }
        };
        let outcome = {
            let resource = resources.as_mut().expect("unavailable channel rejects admission");
            if let Err(error) = job.initialize(resource) {
                JobOutcome { result: Err(JobError::Rejected(error)), cleanup: None }
            } else if self.cancel.signaled() {
                cancelled_outcome()
            } else {
                let deadline_at = job.timeout().map(|duration| embassy_time::Instant::now() + duration);
                let mut context = JobContext::new(id, core::ptr::from_ref(self) as usize);
                let deadline = async {
                    match deadline_at {
                        Some(deadline) => embassy_time::Timer::at(deadline).await,
                        None => core::future::pending::<()>().await,
                    }
                };
                // Poll execution first; cleanup only follows a started run.
                let result = match select(job.run(resource, &mut context), select(self.cancel.wait(), deadline)).await {
                    Either::First(result) => result.map_err(JobError::Rejected),
                    Either::Second(Either::First(())) => Err(JobError::Cancelled),
                    Either::Second(Either::Second(())) => Err(JobError::Deadline),
                };
                // Scope teardown and physical cleanup cannot themselves be interrupted.
                let children = context.close().await;
                let cleanup = job.cleanup(resource, &result).await.and(children);
                JobOutcome { result, cleanup: Some(cleanup) }
            }
        };
        drop(resources);
        self.update(id, outcome);
    }
}

impl<J: Job<Resources = R> + 'static, R: 'static, const N: usize, const H: usize> ChildControl for ChannelStorage<J, R, N, H> {
    fn cancel(&'static self, id: OperationId) {
        let _ = self.channel().cancel(id);
    }

    fn status(&'static self, id: OperationId) -> JobStatus {
        self.channel().status(id).expect("scope retains child history")
    }

    fn poll_finished(&'static self, id: OperationId, cx: &mut core::task::Context<'_>) -> Poll<JobOutcome> {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            let record = state.records.iter().find(|record| record.id == id).expect("scope retains child history");
            if let JobStatus::Finished(outcome) = record.status {
                return Poll::Ready(outcome);
            }
            state.waiters.register(cx.waker());
            Poll::Pending
        })
    }

    fn release(&self, id: OperationId) {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            state.records.iter_mut().find(|record| record.id == id).expect("scope retains child history").observers -= 1;
        });
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn cancelled_outcome() -> JobOutcome {
    JobOutcome { result: Err(JobError::Cancelled), cleanup: None }
}

/* === Tests === */
