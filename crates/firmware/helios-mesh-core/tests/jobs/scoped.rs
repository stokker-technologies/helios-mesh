/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Implement scoped for firmware helios_mesh core tests runtime.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    future::Future,
    pin::{Pin, pin},
    task::{Context, Waker},
};

// Third Party Imports
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

// Local Imports
use helios_mesh_core::jobs::{Channel, ChannelStorage, Duration, Job, JobContext, JobError, JobOutcome, JobStatus, MAX_JOB_CHILDREN};
use helios_mesh_protocol::{
    error::Error,
    node::{OperationId, OperationState},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

type Gate = Signal<CriticalSectionRawMutex, ()>;
type LeafChannel = Channel<Leaf, Counts>;

/* === Structs === */

#[derive(Default)]
struct Counts {
    runs: usize,
    cleanups: usize,
    outcome: Option<Result<(), JobError>>,
}

struct Leaf {
    run_gate: &'static Gate,
    cleanup_gate: &'static Gate,
    cleanup_error: bool,
    timeout: Option<Duration>,
}

struct Parent {
    channels: [LeafChannel; 2],
    run_gate: &'static Gate,
    child_run: &'static Gate,
    child_cleanup: [&'static Gate; 2],
    child_cleanup_error: bool,
    result: Result<(), Error>,
    timeout: Option<Duration>,
}

struct Quick;

struct Sequential {
    child: Channel<Quick, usize>,
}

struct Mixed {
    quick: Channel<Quick, usize>,
    leaf: LeafChannel,
    run: &'static Gate,
    cleanup: &'static Gate,
}

struct SelfSubmit {
    channel: Channel<SelfSubmit, ()>,
}

struct Saturate {
    child: Channel<Leaf, Counts, 16, 32>,
    run: &'static Gate,
    cleanup: &'static Gate,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Job for Leaf {
    type Resources = Counts;
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    async fn run(&mut self, counts: &mut Counts, _: &mut JobContext) -> Result<(), Error> {
        counts.runs += 1;
        self.run_gate.wait().await;
        Ok(())
    }

    async fn cleanup(&mut self, counts: &mut Counts, result: &Result<(), JobError>) -> Result<(), Error> {
        counts.cleanups += 1;
        counts.outcome = Some(*result);
        self.cleanup_gate.wait().await;
        if self.cleanup_error { Err(Error::Unavailable) } else { Ok(()) }
    }
}

impl Job for Parent {
    type Resources = Counts;
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    async fn run(&mut self, counts: &mut Counts, cx: &mut JobContext) -> Result<(), Error> {
        counts.runs += 1;
        for (index, channel) in self.channels.iter().enumerate() {
            // Intentionally drop both caller handles: the scope still owns the children.
            drop(cx.submit(
                channel,
                Leaf {
                    run_gate: self.child_run,
                    cleanup_gate: self.child_cleanup[index],
                    cleanup_error: self.child_cleanup_error,
                    timeout: None,
                },
            )?);
        }
        self.run_gate.wait().await;
        self.result
    }

    fn cleanup(&mut self, counts: &mut Counts, result: &Result<(), JobError>) -> impl core::future::Future<Output = Result<(), Error>> {
        counts.cleanups += 1;
        counts.outcome = Some(*result);
        core::future::ready(Ok(()))
    }
}

impl Job for Quick {
    type Resources = usize;
    fn run(&mut self, count: &mut usize, _: &mut JobContext) -> impl core::future::Future<Output = Result<(), Error>> {
        *count += 1;
        core::future::ready(Ok(()))
    }
}

impl Job for Sequential {
    type Resources = ();
    async fn run(&mut self, (): &mut (), cx: &mut JobContext) -> Result<(), Error> {
        for _ in 0..MAX_JOB_CHILDREN + 2 {
            let child = cx.submit(&self.child, Quick)?;
            assert_eq!(child.finished().await.result, Ok(()));
        }
        Ok(())
    }
}

impl Job for Mixed {
    type Resources = ();
    async fn run(&mut self, (): &mut (), cx: &mut JobContext) -> Result<(), Error> {
        let a = cx.submit(&self.quick, Quick)?;
        let b = cx.submit(&self.leaf, Leaf { run_gate: self.run, cleanup_gate: self.cleanup, cleanup_error: false, timeout: None })?;
        let (a, b) = embassy_futures::join::join(a.finished(), b.finished()).await;
        assert_eq!(a.result, Ok(()));
        assert_eq!(b.result, Ok(()));
        Ok(())
    }
}

impl Job for SelfSubmit {
    type Resources = ();
    fn run(&mut self, (): &mut (), cx: &mut JobContext) -> impl core::future::Future<Output = Result<(), Error>> {
        assert!(matches!(cx.submit(&self.channel, SelfSubmit { channel: self.channel }), Err(Error::Conflict)));
        core::future::ready(Ok(()))
    }
}

impl Job for Saturate {
    type Resources = ();
    fn run(&mut self, (): &mut (), cx: &mut JobContext) -> impl Future<Output = Result<(), Error>> {
        for _ in 0..MAX_JOB_CHILDREN {
            let admission =
                cx.submit(&self.child, Leaf { run_gate: self.run, cleanup_gate: self.cleanup, cleanup_error: false, timeout: None });
            match admission {
                Ok(child) => drop(child),
                Err(error) => return core::future::ready(Err(error)),
            }
        }
        assert!(matches!(
            cx.submit(&self.child, Leaf { run_gate: self.run, cleanup_gate: self.cleanup, cleanup_error: false, timeout: None }),
            Err(Error::Busy)
        ));
        core::future::ready(Ok(()))
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn gate() -> &'static Gate {
    Box::leak(Box::new(Gate::new()))
}

fn channel<J: Job<Resources = R> + 'static, R: 'static>(id: u8, resources: R) -> Channel<J, R> {
    Box::leak(Box::new(ChannelStorage::new(id, Some(resources)))).channel()
}

fn pending<F: Future>(future: Pin<&mut F>) {
    assert!(future.poll(&mut Context::from_waker(Waker::noop())).is_pending());
}

fn finished<J: Job<Resources = R> + 'static, R: 'static>(channel: Channel<J, R>, id: OperationId) -> JobOutcome {
    match channel.status(id).unwrap() {
        JobStatus::Finished(outcome) => outcome,
        other => panic!("expected completion, got {other:?}"),
    }
}

/* === Tests === */

#[test]
fn parent_exit_deadline() {
    for mode in 0..5 {
        let a: LeafChannel = channel(1, Counts::default());
        let b: LeafChannel = channel(2, Counts::default());
        let parent: Channel<Parent, Counts> = channel(3, Counts::default());
        let run = gate();
        let cleanups = [gate(), gate()];
        let operation = parent
            .submit(Parent {
                channels: [a, b],
                run_gate: run,
                child_run: gate(),
                child_cleanup: cleanups,
                child_cleanup_error: mode == 1 || mode == 4,
                result: if mode == 1 { Err(Error::InvalidInput) } else { Ok(()) },
                timeout: if mode == 3 { Some(Duration::from_millis(100)) } else { None },
            })
            .unwrap();
        let mut parent_worker = pin!(parent.run());
        let mut a_worker = pin!(a.run());
        let mut b_worker = pin!(b.run());
        pending(parent_worker.as_mut());
        pending(a_worker.as_mut());
        pending(b_worker.as_mut());

        match mode {
            2 => operation.cancel().unwrap(),
            3 => embassy_time::MockDriver::get().advance(Duration::from_millis(120)),
            _ => run.signal(()),
        }
        pending(parent_worker.as_mut());
        pending(a_worker.as_mut());
        pending(b_worker.as_mut());
        // Both children are cleaning up; parent still owns resources and is nonterminal.
        assert!(matches!(parent.try_resources(), Err(Error::Busy)));
        assert_eq!(parent.status(operation.id()), Some(JobStatus::Running));
        cleanups[0].signal(());
        pending(a_worker.as_mut());
        pending(parent_worker.as_mut());
        assert_eq!(parent.status(operation.id()), Some(JobStatus::Running));
        operation.cancel().unwrap(); // Cannot interrupt teardown or replace the original result.
        cleanups[1].signal(());
        pending(b_worker.as_mut());
        pending(parent_worker.as_mut());

        let expected = match mode {
            1 => Err(JobError::Rejected(Error::InvalidInput)),
            2 => Err(JobError::Cancelled),
            3 => Err(JobError::Deadline),
            _ => Ok(()),
        };
        let outcome = finished(parent, operation.id());
        assert_eq!(outcome.result, expected);
        assert_eq!(outcome.cleanup, Some(if mode == 1 || mode == 4 { Err(Error::Unavailable) } else { Ok(()) }));
        let resource = parent.try_resources().unwrap();
        assert_eq!(resource.cleanups, 1);
        assert_eq!(resource.outcome, Some(expected));
        for leaf in [a, b] {
            let resource = leaf.try_resources().unwrap();
            assert_eq!(resource.runs, 1);
            assert_eq!(resource.cleanups, 1);
            assert_eq!(resource.outcome, Some(Err(JobError::Cancelled)));
        }
    }
}

#[test]
fn scope_rejects_admission() {
    let channel = channel(1, ());
    let operation = channel.submit(SelfSubmit { channel }).unwrap();
    let mut worker = pin!(channel.run());
    pending(worker.as_mut());
    assert_eq!(finished(channel, operation.id()).result, Ok(()));
    assert!(channel.is_idle());
}

#[test]
fn scope_capacity_child() {
    let child = Box::leak(Box::new(ChannelStorage::<Leaf, Counts, 16, 32>::new(1, Some(Counts::default())))).channel();
    let parent: Channel<Saturate, ()> = channel(2, ());
    let operation = parent.submit(Saturate { child, run: gate(), cleanup: gate() }).unwrap();
    let mut parent_worker = pin!(parent.run());
    let mut child_worker = pin!(child.run());
    pending(parent_worker.as_mut());
    pending(child_worker.as_mut()); // All children cancelled while queued: no physical execution.
    pending(parent_worker.as_mut());
    assert_eq!(finished(parent, operation.id()).result, Ok(()));
    assert_eq!(child.try_resources().unwrap().runs, 0);
}

#[test]
fn deadline_execution_only() {
    let channel: LeafChannel = channel(1, Counts::default());
    let cleanup = gate();
    let operation = channel
        .submit(Leaf { run_gate: gate(), cleanup_gate: cleanup, cleanup_error: true, timeout: Some(Duration::from_millis(5)) })
        .unwrap();
    // Time queued, and waiting for external resource access, does not consume execution budget.
    embassy_time::MockDriver::get().advance(Duration::from_millis(10));
    let guard = channel.try_resources().unwrap();
    let mut worker = pin!(channel.run());
    pending(worker.as_mut());
    embassy_time::MockDriver::get().advance(Duration::from_millis(10));
    pending(worker.as_mut());
    drop(guard);
    pending(worker.as_mut());
    assert_eq!(channel.status(operation.id()), Some(JobStatus::Running));
    embassy_time::MockDriver::get().advance(Duration::from_millis(10));
    pending(worker.as_mut());
    operation.cancel().unwrap();
    pending(worker.as_mut());
    assert!(matches!(channel.try_resources(), Err(Error::Busy)));
    cleanup.signal(());
    pending(worker.as_mut());
    assert_eq!(finished(channel, operation.id()), JobOutcome { result: Err(JobError::Deadline), cleanup: Some(Err(Error::Unavailable)) });
    let snapshot = channel.snapshot(operation.id()).unwrap();
    assert_eq!(snapshot.state, OperationState::Failed);
    assert_eq!(snapshot.error, Some(Error::Unavailable));
    assert_eq!(snapshot.cleanup_error, Some(Error::Unavailable));
}

#[test]
fn completed_children_workflows() {
    let child: Channel<Quick, usize> = channel(1, 0);
    // Storage identity, not merely numeric channel ID, defines a self dependency.
    let parent: Channel<Sequential, ()> = channel(1, ());
    let operation = parent.submit(Sequential { child }).unwrap();
    let mut parent_worker = pin!(parent.run());
    let mut child_worker = pin!(child.run());
    for _ in 0..MAX_JOB_CHILDREN + 2 {
        pending(parent_worker.as_mut());
        pending(child_worker.as_mut());
    }
    pending(parent_worker.as_mut());
    assert_eq!(finished(parent, operation.id()).cleanup, Some(Ok(())));
    assert_eq!(*child.try_resources().unwrap(), MAX_JOB_CHILDREN + 2);
}

#[test]
fn heterogeneous_children_workflow() {
    let quick: Channel<Quick, usize> = channel(1, 0);
    let leaf: LeafChannel = channel(2, Counts::default());
    let parent: Channel<Mixed, ()> = channel(3, ());
    let run = gate();
    let cleanup = gate();
    let operation = parent.submit(Mixed { quick, leaf, run, cleanup }).unwrap();
    let mut parent_worker = pin!(parent.run());
    let mut quick_worker = pin!(quick.run());
    let mut leaf_worker = pin!(leaf.run());
    pending(parent_worker.as_mut());
    pending(quick_worker.as_mut());
    pending(leaf_worker.as_mut());
    run.signal(());
    cleanup.signal(());
    pending(leaf_worker.as_mut());
    pending(parent_worker.as_mut());
    assert_eq!(finished(parent, operation.id()), JobOutcome { result: Ok(()), cleanup: Some(Ok(())) });
    assert_eq!(*quick.try_resources().unwrap(), 1);
    assert_eq!(leaf.try_resources().unwrap().cleanups, 1);
}

#[test]
fn failed_second_automatically() {
    let first: LeafChannel = channel(1, Counts::default());
    let unavailable = Box::leak(Box::new(ChannelStorage::<Leaf, Counts>::new(2, None))).channel();
    let parent: Channel<Parent, Counts> = channel(3, Counts::default());
    let operation = parent
        .submit(Parent {
            channels: [first, unavailable],
            run_gate: gate(),
            child_run: gate(),
            child_cleanup: [gate(), gate()],
            child_cleanup_error: false,
            result: Ok(()),
            timeout: None,
        })
        .unwrap();
    let mut parent_worker = pin!(parent.run());
    let mut child_worker = pin!(first.run());
    pending(parent_worker.as_mut());
    assert_eq!(parent.status(operation.id()), Some(JobStatus::Running));
    pending(child_worker.as_mut());
    pending(parent_worker.as_mut());
    assert_eq!(finished(parent, operation.id()), JobOutcome { result: Err(JobError::Rejected(Error::Unavailable)), cleanup: Some(Ok(())) });
    assert_eq!(first.try_resources().unwrap().runs, 0);
    assert!(first.is_idle());
}
