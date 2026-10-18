/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Portable logical stepper motors over step-level hardware drivers.

/* === Modules === */

pub mod plan;
pub mod wing;

/* === Re-Exports === */

pub use plan::{Axis, Direction, Plan, nearest_heading_target, normalize_heading};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Instant, Timer};

// Local Imports
use helios_mesh_protocol::motion::{Mechanics, Microsteps, Power, Timing};
use helios_mesh_protocol::node::OperationId;

/* === Constants === */

pub const COMMAND_DEPTH: usize = 4;
pub const COMPLETION_DEPTH: usize = 8;
const STEP_LATE_TOLERANCE_US: u32 = 100;

/* === Macros === */

/* === Type Aliases === */

pub type Commands = Channel<CriticalSectionRawMutex, Command, COMMAND_DEPTH>;
pub type CommandReceiver = Receiver<'static, CriticalSectionRawMutex, Command, COMMAND_DEPTH>;
pub type Completions = Channel<CriticalSectionRawMutex, Completion, COMPLETION_DEPTH>;
pub type CompletionSender = Sender<'static, CriticalSectionRawMutex, Completion, COMPLETION_DEPTH>;

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Duty(u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    pub mechanics: Mechanics,

    pub power: Power,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Move {
    pub operation: OperationId,

    pub target_microdegrees: i32,

    pub maximum_microdegrees_per_second: u32,

    pub acceleration_microdegrees_per_second_squared: u32,

    pub start: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Completion {
    pub operation: OperationId,

    pub outcome: Outcome,

    pub position_microdegrees: i32,

    pub timing: Timing,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StepTiming {
    pub queued_us: u32,

    pub transaction_us: u32,
}

#[derive(Clone, Copy)]
pub struct Handle {
    commands: Sender<'static, CriticalSectionRawMutex, Command, COMMAND_DEPTH>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rejected;

pub struct Stepper<D: StepperDriver> {
    driver: D,
    config: Config,
    commands: CommandReceiver,
    completions: CompletionSender,
    axis: Axis,
    mode_synchronized: bool,
    active: Option<Active>,
}

struct Active {
    operation: OperationId,
    plan: Plan,
    next_step: Instant,
    started: Instant,
    consecutive_delayed_steps: u32,
    timing: Timing,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Move(Move),

    SetHome { operation: OperationId },

    Stop { operation: OperationId },

    Reconfigure(Config),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Homed,

    Reached { requested_microdegrees: i32 },

    Stopped { interrupted: Option<OperationId> },

    Failed,
}

/* === Traits === */

#[allow(async_fn_in_trait)]
pub trait StepperDriver {
    type Error;

    async fn set_mode(&mut self, microsteps: Microsteps) -> Result<(), Self::Error>;

    async fn step(&mut self, direction: Direction) -> Result<StepTiming, Self::Error>;

    async fn hold(&mut self, duty: Duty) -> Result<(), Self::Error>;

    async fn release(&mut self) -> Result<(), Self::Error>;
}

/* === Implementations === */

impl Duty {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn percent(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    #[must_use]
    pub const fn as_percent(self) -> u8 {
        self.0
    }
}

impl Handle {
    #[must_use]
    pub fn new(commands: &'static Commands) -> Self {
        Self { commands: commands.sender() }
    }

    pub fn try_move(&self, movement: Move) -> Result<(), Rejected> {
        self.commands.try_send(Command::Move(movement)).map_err(|_| Rejected)
    }

    pub fn try_set_home(&self, operation: OperationId) -> Result<(), Rejected> {
        self.commands.try_send(Command::SetHome { operation }).map_err(|_| Rejected)
    }

    pub fn try_stop(&self, operation: OperationId) -> Result<(), Rejected> {
        self.commands.try_send(Command::Stop { operation }).map_err(|_| Rejected)
    }

    #[must_use]
    pub fn can_accept(&self) -> bool {
        !self.commands.is_full()
    }

    pub fn try_reconfigure(&self, config: Config) -> Result<(), Rejected> {
        self.commands.try_send(Command::Reconfigure(config)).map_err(|_| Rejected)
    }
}

impl<D: StepperDriver> Stepper<D> {
    pub fn new(driver: D, config: Config, commands: CommandReceiver, completions: CompletionSender) -> Self {
        Self {
            driver,
            config,
            commands,
            completions,
            axis: Axis::new(steps_per_revolution(config.mechanics)),
            mode_synchronized: false,
            active: None,
        }
    }

    pub async fn run(&mut self) -> ! {
        loop {
            let Some(deadline) = self.active.as_ref().map(|active| active.next_step) else {
                let command = self.commands.receive().await;
                self.handle(command).await;
                continue;
            };
            if Instant::now() < deadline {
                match select(self.commands.receive(), Timer::at(deadline)).await {
                    Either::First(command) => {
                        self.handle(command).await;
                        continue;
                    }
                    Either::Second(()) => {}
                }
            }
            self.service_step().await;
        }
    }

    async fn handle(&mut self, command: Command) {
        if self.active.is_some() {
            match command {
                Command::Stop { operation } => self.stop(operation).await,
                Command::Move(Move { operation, .. }) | Command::SetHome { operation } => {
                    self.complete(operation, Outcome::Failed, Timing::default()).await;
                }
                Command::Reconfigure(_) => defmt::warn!("reconfigure ignored mid-motion"),
            }
            return;
        }
        match command {
            Command::Move(movement) => self.start_move(movement).await,
            Command::SetHome { operation } => self.set_home(operation).await,
            Command::Stop { operation } => self.stop(operation).await,
            Command::Reconfigure(config) => self.reconfigure(config),
        }
    }

    fn reconfigure(&mut self, config: Config) {
        self.config = config;
        self.axis = Axis::new(steps_per_revolution(config.mechanics));
        self.mode_synchronized = false;
    }

    async fn set_home(&mut self, operation: OperationId) {
        self.axis.set_position(0);
        let outcome = if self.settle().await { Outcome::Homed } else { Outcome::Failed };
        self.complete(operation, outcome, Timing::default()).await;
    }

    async fn stop(&mut self, operation: OperationId) {
        let interrupted = self.active.take().map(|active| active.operation);
        let position = normalize_heading(self.axis.position_microdegrees());
        self.axis.set_position(position);
        let outcome = if self.settle().await { Outcome::Stopped { interrupted } } else { Outcome::Failed };
        self.complete(operation, outcome, Timing::default()).await;
    }

    async fn start_move(&mut self, movement: Move) {
        if !self.synchronize_mode().await {
            self.complete(movement.operation, Outcome::Failed, Timing::default()).await;
            return;
        }
        let plan = self.axis.plan_to(
            movement.target_microdegrees,
            movement.maximum_microdegrees_per_second,
            movement.acceleration_microdegrees_per_second_squared,
        );
        if plan.total_steps() == 0 {
            let position = normalize_heading(self.axis.position_microdegrees());
            self.axis.set_position(position);
            let outcome = if self.settle().await {
                Outcome::Reached { requested_microdegrees: movement.target_microdegrees }
            } else {
                Outcome::Failed
            };
            self.complete(movement.operation, outcome, Timing::default()).await;
            return;
        }
        // Energize the windings at movement power before the first step.
        if self.driver.hold(Duty::percent(self.config.power.moving)).await.is_err() {
            let _ = self.driver.release().await;
            self.complete(movement.operation, Outcome::Failed, Timing::default()).await;
            return;
        }
        self.active = Some(Active {
            operation: movement.operation,
            plan,
            next_step: movement.start,
            started: movement.start,
            consecutive_delayed_steps: 0,
            timing: Timing::default(),
        });
    }

    async fn service_step(&mut self) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        let now = Instant::now();
        record_lateness(active, micros(now.duration_since(active.next_step)));
        let direction = physical_direction(active.plan.direction(), self.config.mechanics.reversed);
        // The driver reports its own queued-vs-transaction split (see
        // `StepTiming`) instead of this call being bracketed by two
        // `Instant::now()` calls: a bracket around the whole `.await` cannot
        // tell a wait for a shared, arbitrated output resident apart from
        // the physical transaction it eventually runs, which is exactly the
        // ambiguity that made an unexplained multi-millisecond step stall
        // hard to attribute to a cause.
        let step = self.driver.step(direction).await;
        let update_finished = Instant::now();
        let Ok(timing) = step else {
            let Some(active) = self.active.take() else {
                return;
            };
            let _ = self.driver.release().await;
            self.complete(active.operation, Outcome::Failed, active.timing).await;
            return;
        };
        let Some(active) = self.active.as_mut() else {
            return;
        };
        active.timing.maximum_output_queue_us = active.timing.maximum_output_queue_us.max(timing.queued_us);
        active.timing.maximum_i2c_update_us = active.timing.maximum_i2c_update_us.max(timing.transaction_us);
        active.timing.steps = active.timing.steps.saturating_add(1);
        self.axis.record_step(&mut active.plan);
        if let Some(delay) = active.plan.next_delay_microseconds() {
            let scheduled = active.next_step + Duration::from_micros(delay);
            if scheduled < update_finished {
                active.timing.schedule_rebases = active.timing.schedule_rebases.saturating_add(1);
                active.next_step = update_finished;
            } else {
                active.next_step = scheduled;
            }
            return;
        }
        let Some(finished) = self.active.take() else {
            return;
        };
        let requested = finished.plan.target_microdegrees();
        let position = normalize_heading(self.axis.position_microdegrees());
        self.axis.set_position(position);
        let mut timing = finished.timing;
        timing.duration_ms = u32::try_from(finished.started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let outcome = if self.settle().await { Outcome::Reached { requested_microdegrees: requested } } else { Outcome::Failed };
        self.complete(finished.operation, outcome, timing).await;
    }

    async fn settle(&mut self) -> bool {
        if !self.synchronize_mode().await {
            return false;
        }
        if self.driver.hold(Duty::percent(self.config.power.holding)).await.is_ok() {
            return true;
        }
        let _ = self.driver.release().await;
        false
    }

    async fn synchronize_mode(&mut self) -> bool {
        if self.mode_synchronized {
            return true;
        }
        self.mode_synchronized = self.driver.set_mode(self.config.mechanics.microsteps).await.is_ok();
        self.mode_synchronized
    }

    async fn complete(&mut self, operation: OperationId, outcome: Outcome, timing: Timing) {
        self.completions
            .send(Completion { operation, outcome, position_microdegrees: normalize_heading(self.axis.position_microdegrees()), timing })
            .await;
    }
}

/* === Public Functions === */

pub(crate) fn micros(duration: Duration) -> u32 {
    u32::try_from(duration.as_micros()).unwrap_or(u32::MAX)
}

/* === Private Functions === */

fn steps_per_revolution(mechanics: Mechanics) -> u32 {
    u32::from(mechanics.full_steps).saturating_mul(mechanics.microsteps.count()).max(1)
}

const fn physical_direction(direction: Direction, reversed: bool) -> Direction {
    if !reversed {
        return direction;
    }
    match direction {
        Direction::Positive => Direction::Negative,
        Direction::Negative => Direction::Positive,
    }
}

fn record_lateness(active: &mut Active, lateness_us: u32) {
    active.timing.maximum_lateness_us = active.timing.maximum_lateness_us.max(lateness_us);
    match lateness_us {
        0..=STEP_LATE_TOLERANCE_US => active.consecutive_delayed_steps = 0,
        101..=250 => {
            active.timing.delayed_100_to_250_us = active.timing.delayed_100_to_250_us.saturating_add(1);
        }
        251..=500 => {
            active.timing.delayed_250_to_500_us = active.timing.delayed_250_to_500_us.saturating_add(1);
        }
        501..=1_000 => {
            active.timing.delayed_500_to_1000_us = active.timing.delayed_500_to_1000_us.saturating_add(1);
        }
        _ => {
            active.timing.delayed_over_1000_us = active.timing.delayed_over_1000_us.saturating_add(1);
        }
    }
    if lateness_us > STEP_LATE_TOLERANCE_US {
        active.timing.delayed_steps = active.timing.delayed_steps.saturating_add(1);
        active.consecutive_delayed_steps = active.consecutive_delayed_steps.saturating_add(1);
        active.timing.maximum_consecutive_delayed_steps =
            active.timing.maximum_consecutive_delayed_steps.max(active.consecutive_delayed_steps);
    }
}

/* === Tests === */
