/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Implement jobs for firmware helios_mesh core tests runtime.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

// Third Party Imports
use embassy_futures::join::join;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

// Local Imports
use helios_mesh_core::jobs::{Channel, ChannelStorage, Job, JobContext, JobError, JobOutcome, JobStatus};
use helios_mesh_protocol::error::Error;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type Gate = Signal<CriticalSectionRawMutex, ()>;

/* === Structs === */

#[derive(Default)]
struct Resources {
    value: u8,
    runs: u8,
    cleanups: u8,
}

struct Work {
    invalid: bool,
    run_gate: Option<&'static Gate>,
    cleanup_gate: Option<&'static Gate>,
    result: Result<(), Error>,
    cleanup_result: Result<(), Error>,
}

#[derive(Default)]
struct WakeCount(AtomicUsize);

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl Job for Work {
    type Resources = Resources;

    fn initialize(&self, _: &Resources) -> Result<(), Error> {
        if self.invalid { Err(Error::InvalidInput) } else { Ok(()) }
    }

    async fn run(&mut self, resources: &mut Resources, _: &mut JobContext) -> Result<(), Error> {
        resources.runs += 1;
        resources.value = 1;
        if let Some(gate) = self.run_gate {
            gate.wait().await;
        }
        self.result
    }

    async fn cleanup(&mut self, resources: &mut Resources, _: &Result<(), JobError>) -> Result<(), Error> {
        resources.cleanups += 1;
        resources.value = 0;
        if let Some(gate) = self.cleanup_gate {
            gate.wait().await;
        }
        self.cleanup_result
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn channel<const N: usize, const H: usize>(id: u8) -> Channel<Work, Resources, N, H> {
    Box::leak(Box::new(ChannelStorage::new(id, Some(Resources { value: 7, ..Resources::default() })))).channel()
}

fn gate() -> &'static Gate {
    Box::leak(Box::new(Gate::new()))
}

fn work() -> Work {
    Work { invalid: false, run_gate: None, cleanup_gate: None, result: Ok(()), cleanup_result: Ok(()) }
}

/* === Tests === */

#[test]
fn invalid_queued_resources() {
    let channel = channel::<2, 4>(1);
    let invalid = channel.submit(Work { invalid: true, ..work() }).unwrap();
    let cancelled = channel.submit(work()).unwrap();
    assert!(matches!(channel.submit(work()), Err(Error::Busy)));
    cancelled.cancel().unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let mut worker = pin!(channel.run());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(
        channel.status(invalid.id()),
        Some(JobStatus::Finished(JobOutcome { result: Err(JobError::Rejected(Error::InvalidInput)), cleanup: None }))
    );
    assert_eq!(channel.status(cancelled.id()), Some(JobStatus::Finished(JobOutcome { result: Err(JobError::Cancelled), cleanup: None })));
    let resource = channel.try_resources().unwrap();
    assert_eq!(resource.value, 7);
    assert_eq!(resource.runs, 0);
    assert_eq!(resource.cleanups, 0);
}

#[test]
fn independent_channels_observer() {
    let first = channel::<2, 4>(1);
    let second = channel::<2, 4>(2);
    let finish_a = gate();
    let finish_b = gate();
    let clean_a = gate();
    let a = first.submit(Work { run_gate: Some(finish_a), cleanup_gate: Some(clean_a), ..work() }).unwrap();
    let b = second.submit(Work { run_gate: Some(finish_b), ..work() }).unwrap();
    assert_ne!(a.id(), b.id());
    let mut ar = pin!(first.run());
    let mut br = pin!(second.run());
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(ar.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(br.as_mut().poll(&mut cx), Poll::Pending);
    assert!(first.try_resources().is_err());
    assert!(second.try_resources().is_err());

    let wake_count = Arc::new(WakeCount::default());
    let waker = Waker::from(wake_count.clone());
    let mut observer_cx = Context::from_waker(&waker);
    let mut together = pin!(join(a.finished(), b.finished()));
    assert_eq!(together.as_mut().poll(&mut observer_cx), Poll::Pending);
    finish_a.signal(());
    assert_eq!(ar.as_mut().poll(&mut cx), Poll::Pending);
    assert!(first.try_resources().is_err(), "cleanup still owns the resource");
    assert_eq!(first.status(a.id()), Some(JobStatus::Running));
    assert_eq!(together.as_mut().poll(&mut observer_cx), Poll::Pending);

    finish_b.signal(());
    assert_eq!(br.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(second.try_resources().unwrap().cleanups, 1);
    assert!(wake_count.0.load(Ordering::SeqCst) > 0);
    assert_eq!(together.as_mut().poll(&mut observer_cx), Poll::Pending);
    clean_a.signal(());
    assert_eq!(ar.as_mut().poll(&mut cx), Poll::Pending);
    let success = JobOutcome { result: Ok(()), cleanup: Some(Ok(())) };
    assert_eq!(together.as_mut().poll(&mut observer_cx), Poll::Ready((success, success)));
    assert_eq!(first.try_resources().unwrap().cleanups, 1);
}

#[test]
fn work_survives_drop() {
    let channel = channel::<2, 4>(1);
    let finish = gate();
    let operation = channel.submit(Work { run_gate: Some(finish), ..work() }).unwrap();
    let id = operation.id();
    let mut cx = Context::from_waker(Waker::noop());
    let mut worker = pin!(channel.run());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    {
        let mut waiting = pin!(operation.finished());
        assert_eq!(waiting.as_mut().poll(&mut cx), Poll::Pending);
    }
    drop(operation);
    assert_eq!(channel.status(id), Some(JobStatus::Running));
    finish.signal(());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(channel.status(id), Some(JobStatus::Finished(JobOutcome { result: Ok(()), cleanup: Some(Ok(())) })));
    assert_eq!(channel.try_resources().unwrap().cleanups, 1);
}

#[test]
fn explicit_cancellation_failure() {
    let channel = channel::<2, 4>(1);
    let clean = gate();
    let operation = channel
        .submit(Work { run_gate: Some(gate()), cleanup_gate: Some(clean), cleanup_result: Err(Error::Unavailable), ..work() })
        .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let mut worker = pin!(channel.run());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    operation.cancel().unwrap();
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert!(channel.try_resources().is_err());
    let mut finished = pin!(operation.finished());
    assert_eq!(finished.as_mut().poll(&mut cx), Poll::Pending);
    operation.cancel().unwrap();
    clean.signal(());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(
        finished.as_mut().poll(&mut cx),
        Poll::Ready(JobOutcome { result: Err(JobError::Cancelled), cleanup: Some(Err(Error::Unavailable)) })
    );
    assert_eq!(channel.try_resources().unwrap().cleanups, 1);
}

#[test]
fn retained_handles_history() {
    let channel = channel::<1, 2>(1);
    let mut worker = pin!(channel.run());
    let mut cx = Context::from_waker(Waker::noop());
    let first = channel.submit(work()).unwrap();
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    let second = channel.submit(work()).unwrap();
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(channel.can_submit(), Err(Error::Busy));
    let second_id = second.id();
    drop(second);
    for _ in 0..4 {
        let next = channel.submit(work()).unwrap();
        assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
        drop(next);
    }
    assert_eq!(channel.status(second_id), None);
    assert_eq!(pin!(first.finished()).as_mut().poll(&mut cx), Poll::Ready(JobOutcome { result: Ok(()), cleanup: Some(Ok(())) }));
}

#[test]
fn cancellation_purges_observers() {
    let channel = channel::<2, 4>(1);
    let first = channel.submit(work()).unwrap();
    let second = channel.submit(work()).unwrap();
    let another = channel.operation(first.id()).unwrap();
    let counter_a = Arc::new(WakeCount::default());
    let counter_b = Arc::new(WakeCount::default());
    let waker_a = Waker::from(counter_a.clone());
    let waker_b = Waker::from(counter_b.clone());
    let mut cx_a = Context::from_waker(&waker_a);
    let mut cx_b = Context::from_waker(&waker_b);
    let mut a = pin!(first.finished());
    let mut b = pin!(another.finished());
    assert_eq!(a.as_mut().poll(&mut cx_a), Poll::Pending);
    assert_eq!(b.as_mut().poll(&mut cx_b), Poll::Pending);
    channel.cancel_all();
    assert!(counter_a.0.load(Ordering::SeqCst) > 0);
    assert!(counter_b.0.load(Ordering::SeqCst) > 0);
    let cancelled = JobOutcome { result: Err(JobError::Cancelled), cleanup: None };
    assert_eq!(a.as_mut().poll(&mut cx_a), Poll::Ready(cancelled));
    assert_eq!(b.as_mut().poll(&mut cx_b), Poll::Ready(cancelled));
    assert_eq!(channel.status(second.id()), Some(JobStatus::Finished(cancelled)));
    assert!(channel.is_idle());
    assert_eq!(channel.can_submit(), Ok(()));
    assert_eq!(channel.try_resources().unwrap().cleanups, 0);
}

#[test]
fn unavailable_board_rejected() {
    let channel = Box::leak(Box::new(ChannelStorage::<Work, Resources>::new(1, None))).channel();
    assert!(!channel.is_available());
    assert_eq!(channel.can_submit(), Err(Error::Unavailable));
    assert!(matches!(channel.submit(work()), Err(Error::Unavailable)));
    assert!(matches!(channel.try_resources(), Err(Error::Unavailable)));
}

#[test]
fn cleanup_preserves_primary() {
    for result in [Ok(()), Err(Error::Conflict)] {
        let channel = channel::<1, 2>(1);
        let operation = channel.submit(Work { result, cleanup_result: Err(Error::Unavailable), ..work() }).unwrap();
        let mut cx = Context::from_waker(Waker::noop());
        let mut worker = pin!(channel.run());
        assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(
            pin!(operation.finished()).as_mut().poll(&mut cx),
            Poll::Ready(JobOutcome { result: result.map_err(JobError::Rejected), cleanup: Some(Err(Error::Unavailable)) })
        );
        let snapshot = channel.snapshot(operation.id()).unwrap();
        assert_eq!(snapshot.error, result.err());
        assert_eq!(snapshot.cleanup_error, Some(Error::Unavailable));
    }
}

#[test]
fn partial_admission_cancellation() {
    let first = channel::<1, 4>(1);
    let second = channel::<1, 4>(2);
    let occupied = second.submit(work()).unwrap();
    let a = first.submit(Work { run_gate: Some(gate()), ..work() }).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let mut worker = pin!(first.run());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert!(matches!(second.submit(work()), Err(Error::Busy)));
    a.cancel().unwrap();
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(
        pin!(a.finished()).as_mut().poll(&mut cx),
        Poll::Ready(JobOutcome { result: Err(JobError::Cancelled), cleanup: Some(Ok(())) })
    );
    assert_eq!(first.try_resources().unwrap().cleanups, 1);
    assert_eq!(second.status(occupied.id()), Some(JobStatus::Queued));
    second.cancel_all();
}

#[test]
fn purge_drops_handles() {
    struct Holding {
        _operation: Option<helios_mesh_core::jobs::JobOperation<Holding, Resources, 1, 4>>,
    }
    impl Job for Holding {
        type Resources = Resources;
        fn run(&mut self, _: &mut Resources, _: &mut JobContext) -> impl core::future::Future<Output = Result<(), Error>> {
            core::future::ready(Ok(()))
        }
    }
    let channel = Box::leak(Box::new(ChannelStorage::<Holding, Resources, 1, 4>::new(1, Some(Resources::default())))).channel();
    let operation = channel.submit(Holding { _operation: None }).unwrap();
    let mut worker = pin!(channel.run());
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
    let nested = channel.submit(Holding { _operation: Some(operation) }).unwrap();
    channel.cancel_all();
    assert_eq!(pin!(nested.finished()).as_mut().poll(&mut cx), Poll::Ready(JobOutcome { result: Err(JobError::Cancelled), cleanup: None }));
}
