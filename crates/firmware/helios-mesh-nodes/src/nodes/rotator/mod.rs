/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Independent motor jobs with complete request workflows and no completion events.

/* === Modules === */

pub mod config;
pub mod jobs;

/* === Re-Exports === */

pub use config::Config;

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_time::{Duration, Instant, with_timeout};

#[cfg(target_arch = "arm")]
use {embassy_executor::Spawner, helios_mesh::BootReport, helios_mesh::SatelliteName};

// Local Imports
use helios_mesh::Diagnostics;
use helios_mesh::motor;
use helios_mesh::{Channel, Context, Node, NodeType, OperationId, RequestError as Error};
use helios_mesh::{ChannelId, JournalEvent, motion};
use jobs::{Home, Jobs, Stop, Turn};

/* === Constants === */

pub const CHANNELS: [ChannelId; 2] = [ChannelId(1), ChannelId(2)];

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct MotorChannel {
    motor: motor::Handle,
    completions: &'static motor::Completions,
    configuration: Option<motion::StepperConfiguration>,
    homed: bool,
    position: i32,
    channel: ChannelId,
    diagnostics: Option<&'static Diagnostics>,
    faulted: bool,
}

#[helios_mesh::node]
pub struct Rotator {
    channel_1: Channel<Jobs, MotorChannel>,
    channel_2: Channel<Jobs, MotorChannel>,
}

/* === Enums === */

#[derive(Clone, Copy)]
pub(crate) enum Requested {
    Absolute(i32),
    Relative(i32),
}

/* === Traits === */

/* === Implementations === */

impl MotorChannel {
    pub fn set_diagnostics(&mut self, diagnostics: &'static Diagnostics) {
        self.diagnostics = Some(diagnostics);
    }

    #[cfg(any(target_arch = "arm", test))]
    fn new(motor: motor::Handle, completions: &'static motor::Completions, channel: ChannelId) -> Self {
        Self { motor, completions, channel, configuration: None, homed: false, position: 0, diagnostics: None, faulted: false }
    }

    async fn completion(&mut self, operation: OperationId, timeout: Duration) -> Result<(), Error> {
        let completion = with_timeout(timeout, async {
            loop {
                let completion = self.completions.receive().await;
                if completion.operation == operation {
                    break completion;
                }
            }
        })
        .await
        .map_err(|_| Error::Unavailable)?;
        self.position = completion.position_microdegrees;
        match completion.outcome {
            motor::Outcome::Homed => self.homed = true,
            motor::Outcome::Reached { requested_microdegrees } => {
                if let Some(diagnostics) = self.diagnostics {
                    diagnostics.record(JournalEvent::MotionCompleted {
                        channel: self.channel,
                        operation,
                        requested: motion::Target::Rotational { microdegrees: requested_microdegrees },
                        actual: motion::Target::Rotational { microdegrees: self.position },
                        timing: completion.timing,
                    });
                }
            }
            motor::Outcome::Stopped { interrupted } => {
                self.homed = false;
                if let Some(diagnostics) = self.diagnostics {
                    diagnostics.record(JournalEvent::MotionStopped { channel: self.channel, operation, interrupted });
                }
            }
            motor::Outcome::Failed => {
                self.homed = false;
                return Err(Error::Unavailable);
            }
        }
        Ok(())
    }

    async fn settle_after_cancel(&mut self) -> Result<(), Error> {
        let already_faulted = self.faulted;
        self.faulted = true;
        let stop = OperationId(0);
        self.motor.try_stop(stop).map_err(|_| Error::Busy)?;
        self.completion(stop, Duration::from_secs(30)).await?;
        self.faulted = already_faulted;
        Ok(())
    }

    fn validate(&self, requested: Requested, settings: motion::Settings) -> Result<i32, Error> {
        if self.faulted {
            return Err(Error::Unavailable);
        }
        let config = self.configuration.ok_or(Error::Conflict)?;
        if !self.homed {
            return Err(Error::Conflict);
        }
        if !settings.is_valid_for(config.limits) {
            return Err(Error::InvalidInput);
        }
        match requested {
            Requested::Absolute(value) if (0..=359_999_999).contains(&value) => Ok(motor::nearest_heading_target(self.position, value)),
            Requested::Absolute(_) => Err(Error::InvalidInput),
            Requested::Relative(delta) => self.position.checked_add(delta).ok_or(Error::InvalidInput),
        }
    }
}

impl Rotator {
    #[cfg(target_arch = "arm")]
    pub fn initialize(
        spawner: Spawner, boot: &BootReport, motors: [motor::Handle; 2], completions: [&'static motor::Completions; 2],
    ) -> Self {
        let ready = boot.is_ready(SatelliteName::MotorWing);
        Self::start(
            spawner,
            ready.then(|| MotorChannel::new(motors[0], completions[0], CHANNELS[0])),
            ready.then(|| MotorChannel::new(motors[1], completions[1], CHANNELS[1])),
        )
    }

    #[must_use]
    pub fn motor_channels(&self) -> [Channel<Jobs, MotorChannel>; 2] {
        [self.channel_1, self.channel_2]
    }

    fn index(&self, channel: ChannelId) -> Result<usize, Error> {
        let index = CHANNELS.iter().position(|id| *id == channel).ok_or(Error::NotFound)?;
        if !self.channel(index).is_available() {
            return Err(Error::Unavailable);
        }
        Ok(index)
    }

    fn channel(&self, index: usize) -> &Channel<Jobs, MotorChannel> {
        if index == 0 { &self.channel_1 } else { &self.channel_2 }
    }

    fn submit_turn(
        &self, cx: &Context<'_, Self>, channel: ChannelId, requested: Requested, settings: motion::Settings, start: Instant,
    ) -> Result<helios_mesh::MotionAccepted, Error> {
        let index = self.index(channel)?;
        let target = self.channel(index).try_resources()?.validate(requested, settings)?;
        let operation = self.channel(index).submit(Turn { requested, settings, start, active: None })?;
        record_admission(cx, channel, operation.id(), target);
        Ok(helios_mesh::MotionAccepted { operation: operation.id() })
    }
}

#[helios_mesh::node]
impl Node for Rotator {
    type Config = ();
    const NODE_TYPE: NodeType = NodeType::Rotator;

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn configure(&mut self, _cx: &mut Context<'_, Self>, request: motion::Configure) -> Result<helios_mesh::MotionConfigured, Error> {
        self.index(CHANNELS[0])?;
        let config = Config::try_from(&request.configuration).map_err(|_| Error::InvalidInput)?;
        if self.channels().any(|channel| !channel.is_idle()) {
            return Err(Error::Busy);
        }
        let mut first = self.channel_1.try_resources()?;
        let mut second = self.channel_2.try_resources()?;
        if first.faulted || second.faulted {
            return Err(Error::Unavailable);
        }
        // Both locks exclude other producers; the driver can only free space.
        if !first.motor.can_accept() || !second.motor.can_accept() {
            return Err(Error::Busy);
        }
        for (index, channel) in [&mut *first, &mut *second].into_iter().enumerate() {
            channel.motor.try_reconfigure(config.motor(index)).map_err(|_| Error::Busy)?;
            channel.configuration = Some(config.steppers[index]);
            channel.homed = false;
            channel.position = 0;
        }
        Ok(helios_mesh::MotionConfigured)
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn turn_to(&mut self, cx: &mut Context<'_, Self>, request: motion::MoveTo) -> Result<helios_mesh::MotionAccepted, Error> {
        let motion::Target::Rotational { microdegrees } = request.target else {
            return Err(Error::Unsupported);
        };
        self.submit_turn(
            cx,
            request.channel,
            Requested::Absolute(microdegrees),
            request.settings,
            Instant::now() + Duration::from_millis(20),
        )
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn turn_by(&mut self, cx: &mut Context<'_, Self>, request: motion::MoveRelative) -> Result<helios_mesh::MotionAccepted, Error> {
        let motion::Delta::Rotational { microdegrees } = request.delta else {
            return Err(Error::Unsupported);
        };
        self.submit_turn(
            cx,
            request.channel,
            Requested::Relative(microdegrees),
            request.settings,
            Instant::now() + Duration::from_millis(20),
        )
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn home(&mut self, _cx: &mut Context<'_, Self>, request: motion::SetHome) -> Result<helios_mesh::MotionAccepted, Error> {
        let index = self.index(request.channel)?;
        if self.channel(index).try_resources()?.configuration.is_none() {
            return Err(Error::Conflict);
        }
        let operation = self.channel(index).submit(Home { active: false })?;
        Ok(helios_mesh::MotionAccepted { operation: operation.id() })
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn stop(&mut self, _cx: &mut Context<'_, Self>, request: motion::Stop) -> Result<helios_mesh::MotionAccepted, Error> {
        let index = self.index(request.channel)?;
        self.channel(index).cancel_all();
        let operation = self.channel(index).submit(Stop { active: false })?;
        Ok(helios_mesh::MotionAccepted { operation: operation.id() })
    }

    #[helios_mesh::route]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "Node route signatures are asynchronous by the helios_mesh::node contract."
    )]
    async fn together(
        &mut self, cx: &mut Context<'_, Self>, request: motion::MoveTogether,
    ) -> Result<helios_mesh::MotionAcceptedTogether, Error> {
        if request.moves.len() != 2 {
            return Err(Error::InvalidInput);
        }
        let start = Instant::now() + Duration::from_millis(20);
        for (index, movement) in request.moves.iter().enumerate() {
            if self.index(movement.channel)? != index || !self.channel(index).is_idle() {
                return Err(Error::Busy);
            }
            let motion::Target::Rotational { microdegrees } = movement.target else {
                return Err(Error::Unsupported);
            };
            self.channel(index).try_resources()?.validate(Requested::Absolute(microdegrees), movement.settings)?;
            self.channel(index).can_submit()?;
        }
        let mut operations = heapless::Vec::new();
        for (index, movement) in request.moves.iter().enumerate() {
            let motion::Target::Rotational { microdegrees } = movement.target else { unreachable!() };
            let target = self.channel(index).try_resources()?.validate(Requested::Absolute(microdegrees), movement.settings)?;
            let operation = self.channel(index).submit(Turn {
                requested: Requested::Absolute(microdegrees),
                settings: movement.settings,
                start,
                active: None,
            })?;
            record_admission(cx, movement.channel, operation.id(), target);
            let _ = operations.push(operation.id());
        }
        Ok(helios_mesh::MotionAcceptedTogether { operations })
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn record_admission(cx: &Context<'_, Rotator>, channel: ChannelId, operation: OperationId, microdegrees: i32) {
    if let (Some(helios_mesh::Origin::Lora { peer, .. }), Some(request_id)) = (cx.origin(), cx.request_id()) {
        cx.diagnostics().record(JournalEvent::MotionAccepted {
            peer,
            request_id,
            channel,
            operation,
            target: motion::Target::Rotational { microdegrees },
        });
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::{
        future::Future,
        pin::pin,
        task::{Context as TaskContext, Poll, Waker},
    };
    use helios_mesh::__private::ChannelStorage;
    use helios_mesh::{JobError, JobOutcome, JobStatus};

    fn configuration() -> motion::StepperConfiguration {
        motion::StepperConfiguration {
            mechanics: motion::Mechanics { full_steps: 200, microsteps: motion::Microsteps::Four, reversed: false },
            limits: motion::Limits { speed: 90, acceleration: 180 },
            power: motion::Power { moving: 20, holding: 5 },
        }
    }

    #[test]
    fn cancellation_waits_failure() {
        for failed_stop in [false, true] {
            let commands = std::boxed::Box::leak(std::boxed::Box::new(motor::Commands::new()));
            let completions = std::boxed::Box::leak(std::boxed::Box::new(motor::Completions::new()));
            let mut channel = MotorChannel::new(motor::Handle::new(commands), completions, CHANNELS[0]);
            channel.configuration = Some(configuration());
            channel.homed = true;
            let handle = std::boxed::Box::leak(std::boxed::Box::new(ChannelStorage::<Jobs, MotorChannel>::new(1, Some(channel)))).channel();
            let operation = handle
                .submit(Turn {
                    requested: Requested::Absolute(90_000_000),
                    settings: motion::Settings::default(),
                    start: Instant::from_ticks(0),
                    active: None,
                })
                .unwrap();
            let id = operation.id();
            let mut future = pin!(handle.run());
            let mut cx = TaskContext::from_waker(Waker::noop());
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
            assert!(matches!(commands.try_receive(), Ok(motor::Command::Move(_))));
            handle.cancel(id).unwrap();
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
            assert!(matches!(commands.try_receive(), Ok(motor::Command::Stop { operation: OperationId(0) })));
            completions
                .try_send(motor::Completion {
                    operation: id,
                    outcome: motor::Outcome::Reached { requested_microdegrees: 90_000_000 },
                    position_microdegrees: 90_000_000,
                    timing: motion::Timing::default(),
                })
                .unwrap();
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
            assert!(handle.try_resources().is_err());
            completions
                .try_send(motor::Completion {
                    operation: OperationId(0),
                    outcome: if failed_stop { motor::Outcome::Failed } else { motor::Outcome::Stopped { interrupted: Some(id) } },
                    position_microdegrees: 45_000_000,
                    timing: motion::Timing::default(),
                })
                .unwrap();
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
            assert_eq!(
                handle.status(id),
                Some(JobStatus::Finished(JobOutcome {
                    result: Err(JobError::Cancelled),
                    cleanup: Some(if failed_stop { Err(Error::Unavailable) } else { Ok(()) })
                }))
            );
            let resource = handle.try_resources().unwrap();
            assert!(!resource.homed);
            assert_eq!(resource.faulted, failed_stop);
        }
    }

    #[test]
    fn invalid_unhomed_driver() {
        let commands = std::boxed::Box::leak(std::boxed::Box::new(motor::Commands::new()));
        let completions = std::boxed::Box::leak(std::boxed::Box::new(motor::Completions::new()));
        let mut channel = MotorChannel::new(motor::Handle::new(commands), completions, CHANNELS[0]);
        let settings = motion::Settings::default();
        assert_eq!(channel.validate(Requested::Absolute(0), settings), Err(Error::Conflict));
        channel.configuration = Some(configuration());
        assert_eq!(channel.validate(Requested::Absolute(0), settings), Err(Error::Conflict));
        channel.homed = true;
        assert_eq!(channel.validate(Requested::Absolute(-1), settings), Err(Error::InvalidInput));
        assert_eq!(channel.validate(Requested::Absolute(360_000_000), settings), Err(Error::InvalidInput));
        assert!(commands.try_receive().is_err());
    }
    #[test]
    fn node_enumerates_workers() {
        struct SetFlag;
        impl helios_mesh::Job for SetFlag {
            type Resources = bool;
            async fn run(&mut self, resource: &mut bool, _: &mut helios_mesh::JobContext) -> Result<(), Error> {
                *resource = true;
                Ok(())
            }
        }

        struct SetCount;
        impl helios_mesh::Job for SetCount {
            type Resources = u32;
            async fn run(&mut self, resource: &mut u32, _: &mut helios_mesh::JobContext) -> Result<(), Error> {
                *resource = 1;
                Ok(())
            }
        }

        #[helios_mesh::node]
        struct Mixed {
            flag: Channel<SetFlag, bool>,
            count: Channel<SetCount, u32>,
        }

        #[helios_mesh::node]
        impl Node for Mixed {
            type Config = ();
            const NODE_TYPE: NodeType = NodeType::Sidecar;
        }

        let flag = std::boxed::Box::leak(std::boxed::Box::new(ChannelStorage::<SetFlag, bool>::new(1, Some(false)))).channel();
        let count = std::boxed::Box::leak(std::boxed::Box::new(ChannelStorage::<SetCount, u32>::new(2, None))).channel();
        let mut node = Mixed { flag, count };
        let views: std::vec::Vec<_> = node.channels().collect();
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].name(), "flag");
        assert_eq!(views[0].id(), ChannelId(1));
        assert!(views[0].is_available());
        assert_eq!(views[1].name(), "count");
        assert_eq!(views[1].id(), ChannelId(2));
        assert!(!views[1].is_available());
        assert!(views.iter().all(|channel| channel.is_idle()));

        let operation = flag.submit(SetFlag).unwrap();
        assert_eq!(views[0].queued(), 1);
        assert!(!views[0].is_idle());
        assert_eq!(views[0].status(operation.id()), Some(helios_mesh::JobStatus::Queued));
        views[0].cancel_all();
        assert_eq!(views[0].queued(), 0);
        assert!(views[0].is_idle());
        assert!(!*flag.try_resources().unwrap());
        drop(views);
        assert_eq!(node.operation_status(operation.id()).unwrap().channel, ChannelId(1));
        node.cancel_operation(operation.id()).unwrap();
        assert_eq!(node.cancel_operation(OperationId(u32::MAX)), Err(Error::NotFound));

        assert_eq!(crate::nodes::gateway::Gateway.channels().count(), 0);
    }

    #[test]
    fn default_node_channels() {
        fn channel(id: u8) -> Channel<Jobs, MotorChannel> {
            let commands = std::boxed::Box::leak(std::boxed::Box::new(motor::Commands::new()));
            let completions = std::boxed::Box::leak(std::boxed::Box::new(motor::Completions::new()));
            std::boxed::Box::leak(std::boxed::Box::new(ChannelStorage::new(
                id,
                Some(MotorChannel::new(motor::Handle::new(commands), completions, ChannelId(id))),
            )))
            .channel()
        }
        let mut node = Rotator { channel_1: channel(1), channel_2: channel(2) };
        for (index, channel) in node.motor_channels().into_iter().enumerate() {
            let operation = channel.submit(Jobs::Home(Home { active: false })).unwrap();
            let snapshot = node.operation_status(operation.id()).unwrap();
            assert_eq!(snapshot.channel, CHANNELS[index]);
            node.cancel_operation(operation.id()).unwrap();
            let mut worker = pin!(channel.run());
            let mut cx = TaskContext::from_waker(Waker::noop());
            assert_eq!(worker.as_mut().poll(&mut cx), Poll::Pending);
            assert_eq!(node.operation_status(operation.id()).unwrap().state, helios_mesh::OperationState::Stopped);
        }
        assert_eq!(node.cancel_operation(OperationId(u32::MAX)), Err(Error::NotFound));
    }
}
