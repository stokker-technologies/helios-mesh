/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Adafruit DC Motor + Stepper `FeatherWing` owner and its stepper ports.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, with_timeout};
use embedded_hal_async::i2c::I2c;
use pwm_pca9685::{Address, ChannelOnOffControl, Error, Pca9685};

// Local Imports
use super::{Direction, Duty, StepTiming, StepperDriver, micros};
use crate::bus::i2c::I2cBatch;
use hnm_protocol::motion::Microsteps;

/* === Constants === */

pub const ADDRESS: u8 = 0x60;
pub const MAXIMUM_I2C_FREQUENCY_HZ: u32 = 400_000;
pub const STEPPER_PORTS: usize = 2;
const PWM_PRESCALE_1_526_HZ: u8 = 3;
const MAXIMUM_DUTY: u16 = 4095;
const PWM_CHANNELS: [usize; 4] = [2, 7, 8, 13];
const DIRECTION_CHANNELS: [usize; 8] = [3, 4, 5, 6, 9, 10, 11, 12];
const FIRST_CHANNEL_REGISTER: u8 = 0x06;
const BYTES_PER_CHANNEL: usize = 4;
const FULL_ON_OFF: u8 = 0b0001_0000;
const TRANSACTION_TIMEOUT: Duration = Duration::from_millis(250);
const MICROSTEP_CURVE_4: [u16; 5] = [0, 1567, 2896, 3784, 4095];
const MICROSTEP_CURVE_8: [u16; 9] = [0, 799, 1567, 2275, 2896, 3405, 3784, 4016, 4095];

/* === Macros === */

/* === Type Aliases === */

type Commits = Channel<CriticalSectionRawMutex, Commit, STEPPER_PORTS>;
type Acks = [Signal<CriticalSectionRawMutex, Result<StepTiming, OutputUnavailable>>; STEPPER_PORTS];

/* === Structs === */

pub struct Wing<I2C> {
    shared: Shared,
    boundary: Boundary<I2C>,
}

pub struct WingRunner<I2C: 'static> {
    shared: &'static Shared,
    boundary: &'static mut Boundary<I2C>,
}

pub struct StepperPort {
    shared: &'static Shared,
    slot: StepperSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("motor output is unavailable")]
pub struct OutputUnavailable;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("batch contains conflicting motor actions")]
pub struct BatchConflict;

#[derive(Clone, Copy)]
pub struct Batch {
    dc: [Option<DcDrive>; 4],
    steppers: [Option<StepperAction>; STEPPER_PORTS],
}

struct Shared {
    commits: Commits,
    acks: Acks,
}

#[derive(Clone, Copy)]
struct Commit {
    port: usize,
    action: StepperAction,

    enqueued: Instant,
}

struct Boundary<I2C> {
    i2c: I2C,
    state: Option<WingState<I2C>>,
    timeout: Duration,
}

struct WingState<I2C> {
    i2c: I2C,
    pair_modes: [PairMode; 2],
    dc: [DcDrive; 4],
    steppers: [StepperState; STEPPER_PORTS],
    outputs: [ChannelOnOffControl; 16],
    outputs_synchronized: bool,
}

#[derive(Clone, Copy)]
struct StepperState {
    phase: u8,
    microsteps: Microsteps,
    duty_percent: u8,
}

#[derive(Clone, Copy)]
struct Run {
    data: [u8; 1 + BYTES_PER_CHANNEL * 16],
    length: usize,
}

struct DcPins {
    pwm: usize,
    positive: usize,
    negative: usize,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepperSlot {
    One,

    Two,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcPort {
    One,

    Two,

    Three,

    Four,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcDrive {
    Coast,

    Forward(u8),

    Reverse(u8),

    Brake,
}

#[derive(Clone, Copy)]
enum StepperAction {
    SetMode(Microsteps),
    Step(Direction),
    Hold(Duty),
    Release,
}

#[derive(Clone, Copy)]
enum PairMode {
    Dc,
    Stepper,
}

/* === Traits === */

/* === Implementations === */

impl StepperSlot {
    const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }

    const fn from_index(index: usize) -> Self {
        if index == 0 { Self::One } else { Self::Two }
    }
}

impl DcPort {
    const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
            Self::Three => 2,
            Self::Four => 3,
        }
    }

    const fn pair(self) -> usize {
        self.index() / 2
    }
}

impl Batch {
    #[must_use]
    pub const fn new() -> Self {
        Self { dc: [None; 4], steppers: [None; STEPPER_PORTS] }
    }

    pub fn drive_dc(&mut self, port: DcPort, drive: DcDrive) -> Result<(), BatchConflict> {
        if self.steppers[port.pair()].is_some() {
            return Err(BatchConflict);
        }
        self.dc[port.index()] = Some(drive);
        Ok(())
    }

    pub fn set_mode(&mut self, slot: StepperSlot, microsteps: Microsteps) -> Result<(), BatchConflict> {
        self.stepper(slot, StepperAction::SetMode(microsteps))
    }

    pub fn step(&mut self, slot: StepperSlot, direction: Direction) -> Result<(), BatchConflict> {
        self.stepper(slot, StepperAction::Step(direction))
    }

    pub fn hold(&mut self, slot: StepperSlot, duty: Duty) -> Result<(), BatchConflict> {
        self.stepper(slot, StepperAction::Hold(duty))
    }

    pub fn release(&mut self, slot: StepperSlot) -> Result<(), BatchConflict> {
        self.stepper(slot, StepperAction::Release)
    }

    fn stepper(&mut self, slot: StepperSlot, action: StepperAction) -> Result<(), BatchConflict> {
        let index = slot.index();
        if self.dc[index * 2..index * 2 + 2].iter().any(Option::is_some) {
            return Err(BatchConflict);
        }
        self.steppers[index] = Some(action);
        Ok(())
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

impl Shared {
    const fn new() -> Self {
        Self { commits: Channel::new(), acks: [Signal::new(), Signal::new()] }
    }
}

impl<I2C, E> Wing<I2C>
where
    I2C: I2c<Error = E> + I2cBatch<Error = E> + Clone,
{
    pub async fn connect(i2c: I2C) -> crate::Result<Self, E> {
        let boundary = Boundary::connect(i2c, TRANSACTION_TIMEOUT).await?;
        Ok(Self { shared: Shared::new(), boundary })
    }

    pub fn into_steppers(&'static mut self) -> (WingRunner<I2C>, [StepperPort; STEPPER_PORTS]) {
        let Self { shared, boundary } = self;
        let shared: &'static Shared = shared;
        (WingRunner { shared, boundary }, [StepperPort { shared, slot: StepperSlot::One }, StepperPort { shared, slot: StepperSlot::Two }])
    }
}

impl<I2C, E> WingRunner<I2C>
where
    I2C: I2c<Error = E> + I2cBatch<Error = E> + Clone,
{
    pub async fn run(&mut self) -> ! {
        loop {
            self.service().await;
        }
    }

    async fn service(&mut self) {
        let first = self.shared.commits.receive().await;
        let dequeued = Instant::now();
        let mut pending: [Option<StepperAction>; STEPPER_PORTS] = [None; STEPPER_PORTS];
        let mut queued_us: [u32; STEPPER_PORTS] = [0; STEPPER_PORTS];
        pending[first.port] = Some(first.action);
        queued_us[first.port] = micros(dequeued.duration_since(first.enqueued));
        while let Ok(commit) = self.shared.commits.try_receive() {
            pending[commit.port] = Some(commit.action);
            queued_us[commit.port] = micros(dequeued.duration_since(commit.enqueued));
        }
        let mut batch = Batch::new();
        for (port, action) in pending.iter().enumerate() {
            if let Some(action) = action {
                let _ = batch.stepper(StepperSlot::from_index(port), *action);
            }
        }
        let transaction_started = Instant::now();
        let result = self.boundary.commit(batch).await;
        let transaction_us = micros(Instant::now().duration_since(transaction_started));
        for (port, action) in pending.iter().enumerate() {
            if action.is_some() {
                let outcome = result.map(|()| StepTiming { queued_us: queued_us[port], transaction_us });
                self.shared.acks[port].signal(outcome);
            }
        }
    }
}

impl StepperPort {
    async fn commit(&mut self, action: StepperAction) -> Result<StepTiming, OutputUnavailable> {
        let ack = &self.shared.acks[self.slot.index()];
        ack.reset();
        self.shared.commits.send(Commit { port: self.slot.index(), action, enqueued: Instant::now() }).await;
        ack.wait().await
    }
}

impl StepperDriver for StepperPort {
    type Error = OutputUnavailable;

    async fn set_mode(&mut self, microsteps: Microsteps) -> Result<(), Self::Error> {
        self.commit(StepperAction::SetMode(microsteps)).await.map(|_| ())
    }

    async fn step(&mut self, direction: Direction) -> Result<StepTiming, Self::Error> {
        self.commit(StepperAction::Step(direction)).await
    }

    async fn hold(&mut self, duty: Duty) -> Result<(), Self::Error> {
        self.commit(StepperAction::Hold(duty)).await.map(|_| ())
    }

    async fn release(&mut self) -> Result<(), Self::Error> {
        self.commit(StepperAction::Release).await.map(|_| ())
    }
}

impl<I2C, E> Boundary<I2C>
where
    I2C: I2c<Error = E> + I2cBatch<Error = E> + Clone,
{
    async fn connect(i2c: I2C, timeout: Duration) -> crate::Result<Self, E> {
        let state =
            with_timeout(timeout, WingState::new(i2c.clone())).await.map_err(|_| crate::Error::Timeout)?.map_err(crate::Error::Device)?;
        Ok(Self { i2c, state: Some(state), timeout })
    }

    async fn commit(&mut self, batch: Batch) -> Result<(), OutputUnavailable> {
        self.reconnect().await;
        let result = {
            let Some(state) = self.state.as_mut() else {
                return Err(OutputUnavailable);
            };
            with_timeout(self.timeout, state.commit(batch)).await
        };
        if matches!(result, Ok(Ok(()))) {
            return Ok(());
        }
        self.state = None;
        Err(OutputUnavailable)
    }

    async fn reconnect(&mut self) {
        if self.state.is_some() {
            return;
        }
        self.state = match with_timeout(self.timeout, WingState::new(self.i2c.clone())).await {
            Ok(Ok(state)) => Some(state),
            Ok(Err(_)) | Err(_) => None,
        };
    }
}

impl<I2C, E> WingState<I2C>
where
    I2C: I2c<Error = E> + I2cBatch<Error = E>,
{
    async fn new(i2c: I2C) -> Result<Self, Error<E>> {
        let mut pwm = Pca9685::new(i2c, Address::from(ADDRESS))?;
        let outputs = [ChannelOnOffControl::default(); 16];
        pwm.set_prescale(PWM_PRESCALE_1_526_HZ).await?;
        pwm.enable().await?;
        pwm.set_all_channels(&outputs).await?;
        Ok(Self {
            i2c: pwm.destroy(),
            pair_modes: [PairMode::Stepper; 2],
            dc: [DcDrive::Coast; 4],
            steppers: [StepperState { phase: 0, microsteps: Microsteps::Four, duty_percent: 0 }; STEPPER_PORTS],
            outputs,
            outputs_synchronized: true,
        })
    }

    async fn commit(&mut self, batch: Batch) -> Result<(), Error<E>> {
        self.apply_batch(batch);
        let desired = self.desired_outputs();
        if !self.outputs_synchronized {
            return self.synchronize_outputs(desired).await;
        }
        if let Err(error) = self.write_changed_channels(&desired).await {
            self.outputs_synchronized = false;
            return Err(error);
        }
        self.outputs = desired;
        Ok(())
    }

    fn apply_batch(&mut self, batch: Batch) {
        for (index, action) in batch.steppers.into_iter().enumerate() {
            let Some(action) = action else { continue };
            self.pair_modes[index] = PairMode::Stepper;
            self.dc[index * 2] = DcDrive::Coast;
            self.dc[index * 2 + 1] = DcDrive::Coast;
            let state = &mut self.steppers[index];
            match action {
                StepperAction::SetMode(microsteps) => select_microsteps(state, microsteps),
                StepperAction::Step(direction) => {
                    let phase_count = microstep_count(state.microsteps) * 4;
                    state.phase = match direction {
                        Direction::Positive => state.phase.wrapping_add(1) % phase_count,
                        Direction::Negative => state.phase.wrapping_add(phase_count - 1) % phase_count,
                    };
                }
                StepperAction::Hold(duty) => state.duty_percent = duty.as_percent(),
                StepperAction::Release => state.duty_percent = 0,
            }
        }
        for (index, drive) in batch.dc.into_iter().enumerate() {
            let Some(drive) = drive else { continue };
            let pair = index / 2;
            if matches!(self.pair_modes[pair], PairMode::Stepper) {
                self.dc[pair * 2] = DcDrive::Coast;
                self.dc[pair * 2 + 1] = DcDrive::Coast;
            }
            self.pair_modes[pair] = PairMode::Dc;
            self.steppers[pair].duty_percent = 0;
            self.dc[index] = drive;
        }
    }

    fn desired_outputs(&self) -> [ChannelOnOffControl; 16] {
        let mut desired = [ChannelOnOffControl::default(); 16];
        for pair in 0..2 {
            match self.pair_modes[pair] {
                PairMode::Stepper => apply_stepper(&mut desired, pair, self.steppers[pair]),
                PairMode::Dc => {
                    apply_dc(&mut desired, pair * 2, self.dc[pair * 2]);
                    apply_dc(&mut desired, pair * 2 + 1, self.dc[pair * 2 + 1]);
                }
            }
        }
        desired
    }

    async fn write_changed_channels(&mut self, desired: &[ChannelOnOffControl; 16]) -> Result<(), Error<E>> {
        let mut pwm_runs = [Run::new(); 4];
        let pwm_count = self.diff_runs(&PWM_CHANNELS, desired, &mut pwm_runs);
        let mut direction_runs = [Run::new(); 4];
        let direction_count = self.diff_runs(&DIRECTION_CHANNELS, desired, &mut direction_runs);

        let mut writes: heapless::Vec<&[u8], 8> = heapless::Vec::new();
        for run in &pwm_runs[..pwm_count] {
            let _ = writes.push(run.bytes());
        }
        for run in &direction_runs[..direction_count] {
            let _ = writes.push(run.bytes());
        }
        if writes.is_empty() {
            return Ok(());
        }
        self.i2c.write_batch(ADDRESS, &writes).await.map_err(Error::I2C)
    }

    fn diff_runs(&self, candidates: &[usize], desired: &[ChannelOnOffControl; 16], runs: &mut [Run; 4]) -> usize {
        let mut count = 0;
        let mut cursor = 0;
        while cursor < candidates.len() {
            while cursor < candidates.len() && self.outputs[candidates[cursor]] == desired[candidates[cursor]] {
                cursor += 1;
            }
            if cursor == candidates.len() {
                break;
            }
            let start = candidates[cursor];
            let mut end = start;
            cursor += 1;
            while cursor < candidates.len()
                && candidates[cursor] == end + 1
                && self.outputs[candidates[cursor]] != desired[candidates[cursor]]
            {
                end = candidates[cursor];
                cursor += 1;
            }
            runs[count].encode(start, end, desired);
            count += 1;
        }
        count
    }

    async fn write_contiguous_channels(&mut self, start: usize, end: usize, desired: &[ChannelOnOffControl; 16]) -> Result<(), Error<E>> {
        let mut run = Run::new();
        run.encode(start, end, desired);
        self.i2c.write(ADDRESS, run.bytes()).await.map_err(Error::I2C)
    }

    async fn synchronize_outputs(&mut self, desired: [ChannelOnOffControl; 16]) -> Result<(), Error<E>> {
        self.outputs_synchronized = false;
        let released = [ChannelOnOffControl::default(); 16];
        self.write_contiguous_channels(0, 15, &released).await?;
        let mut directions = desired;
        for channel in PWM_CHANNELS {
            directions[channel] = ChannelOnOffControl::default();
        }
        self.write_contiguous_channels(0, 15, &directions).await?;
        self.write_contiguous_channels(0, 15, &desired).await?;
        self.outputs = desired;
        self.outputs_synchronized = true;
        Ok(())
    }
}

impl Run {
    const fn new() -> Self {
        Self { data: [0; 1 + BYTES_PER_CHANNEL * 16], length: 0 }
    }

    fn encode(&mut self, start: usize, end: usize, desired: &[ChannelOnOffControl; 16]) {
        self.data[0] = FIRST_CHANNEL_REGISTER + u8::try_from(start * BYTES_PER_CHANNEL).unwrap_or(0);
        for (offset, value) in desired[start..=end].iter().enumerate() {
            let base = 1 + offset * BYTES_PER_CHANNEL;
            encode_channel(&mut self.data[base..base + BYTES_PER_CHANNEL], *value);
        }
        self.length = 1 + (end - start + 1) * BYTES_PER_CHANNEL;
    }

    fn bytes(&self) -> &[u8] {
        &self.data[..self.length]
    }
}

/* === Public Functions === */

/* === Private Functions === */

const fn microstep_count(microsteps: Microsteps) -> u8 {
    match microsteps {
        Microsteps::Four => 4,
        Microsteps::Eight => 8,
    }
}

const fn microstep_curve(microsteps: Microsteps) -> &'static [u16] {
    match microsteps {
        Microsteps::Four => &MICROSTEP_CURVE_4,
        Microsteps::Eight => &MICROSTEP_CURVE_8,
    }
}

fn select_microsteps(state: &mut StepperState, microsteps: Microsteps) {
    if state.microsteps == microsteps {
        return;
    }
    state.phase = state.phase * microstep_count(microsteps) / microstep_count(state.microsteps);
    state.microsteps = microsteps;
}

fn encode_channel(bytes: &mut [u8], value: ChannelOnOffControl) {
    let [on_low, on_high] = value.on.to_le_bytes();
    let [off_low, off_high] = value.off.to_le_bytes();
    bytes[0] = on_low;
    bytes[1] = on_high | (FULL_ON_OFF * u8::from(value.full_on));
    bytes[2] = off_low;
    bytes[3] = off_high | (FULL_ON_OFF * u8::from(value.full_off));
}

fn apply_stepper(outputs: &mut [ChannelOnOffControl; 16], index: usize, state: StepperState) {
    if state.duty_percent == 0 {
        return;
    }
    let (first, second) = if index == 0 { (0, 1) } else { (2, 3) };
    let count = microstep_count(state.microsteps);
    let curve = microstep_curve(state.microsteps);
    let phase = state.phase % (count * 4);
    let offset = usize::from(phase % count);
    let complement = usize::from(count) - offset;
    let (a_duty, b_duty, a_positive, b_positive) = match phase / count {
        0 => (curve[complement], curve[offset], true, true),
        1 => (curve[offset], curve[complement], false, true),
        2 => (curve[complement], curve[offset], false, false),
        _ => (curve[offset], curve[complement], true, false),
    };
    apply_winding(outputs, first, a_duty, state.duty_percent, a_positive);
    apply_winding(outputs, second, b_duty, state.duty_percent, b_positive);
}

fn apply_dc(outputs: &mut [ChannelOnOffControl; 16], index: usize, drive: DcDrive) {
    let pins = dc_pins(index);
    match drive {
        DcDrive::Coast => {}
        DcDrive::Forward(duty) => {
            set_pwm(outputs, pins.pwm, scale_duty(MAXIMUM_DUTY, duty));
            set_high(outputs, pins.positive);
        }
        DcDrive::Reverse(duty) => {
            set_pwm(outputs, pins.pwm, scale_duty(MAXIMUM_DUTY, duty));
            set_high(outputs, pins.negative);
        }
        DcDrive::Brake => {
            set_pwm(outputs, pins.pwm, MAXIMUM_DUTY);
            set_high(outputs, pins.positive);
            set_high(outputs, pins.negative);
        }
    }
}

fn apply_winding(outputs: &mut [ChannelOnOffControl; 16], dc_index: usize, duty: u16, percent: u8, positive: bool) {
    let pins = dc_pins(dc_index);
    set_pwm(outputs, pins.pwm, scale_duty(duty, percent));
    set_high(outputs, if positive { pins.positive } else { pins.negative });
}

const fn dc_pins(index: usize) -> DcPins {
    match index {
        0 => DcPins { pwm: 8, positive: 10, negative: 9 },
        1 => DcPins { pwm: 13, positive: 11, negative: 12 },
        2 => DcPins { pwm: 2, positive: 4, negative: 3 },
        _ => DcPins { pwm: 7, positive: 5, negative: 6 },
    }
}

fn scale_duty(duty: u16, percent: u8) -> u16 {
    let scaled = u32::from(duty) * u32::from(percent.min(100)) / 100;
    u16::try_from(scaled).unwrap_or(MAXIMUM_DUTY)
}

fn set_high(outputs: &mut [ChannelOnOffControl; 16], channel: usize) {
    outputs[channel].full_on = true;
}

fn set_pwm(outputs: &mut [ChannelOnOffControl; 16], channel: usize, duty: u16) {
    outputs[channel].off = duty;
}

/* === Tests === */

#[cfg(test)]
mod tests {
    extern crate std;

    use core::cell::RefCell;
    use core::convert::Infallible;

    use embassy_futures::block_on;
    use embassy_futures::join::{join, join3};
    use embedded_hal::i2c::{ErrorType, Operation};
    use std::boxed::Box;
    use std::rc::Rc;
    use std::vec::Vec;

    use super::*;

    #[derive(Clone)]
    struct RecordingI2c(Rc<RefCell<Vec<Vec<u8>>>>);

    impl ErrorType for RecordingI2c {
        type Error = Infallible;
    }

    impl I2c for RecordingI2c {
        fn transaction(
            &mut self, _address: u8, operations: &mut [Operation<'_>],
        ) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            for operation in operations.iter_mut() {
                if let Operation::Write(bytes) = operation {
                    self.0.borrow_mut().push(bytes.to_vec());
                }
            }
            core::future::ready(Ok(()))
        }
    }

    impl I2cBatch for RecordingI2c {
        async fn write_batch(&mut self, address: u8, writes: &[&[u8]]) -> Result<(), Self::Error> {
            for write in writes {
                self.write(address, write).await?;
            }
            Ok(())
        }
    }

    #[test]
    fn mixed_batches_ports() {
        let mut batch = Batch::new();
        batch.step(StepperSlot::One, Direction::Positive).unwrap();
        assert_eq!(batch.drive_dc(DcPort::Two, DcDrive::Forward(50)), Err(BatchConflict));
    }

    #[test]
    fn stepper_can_motors() {
        let mut batch = Batch::new();
        batch.step(StepperSlot::One, Direction::Positive).unwrap();
        batch.drive_dc(DcPort::Three, DcDrive::Forward(40)).unwrap();
        batch.drive_dc(DcPort::Four, DcDrive::Reverse(40)).unwrap();
    }

    #[test]
    fn dc_drive_channels() {
        let mut outputs = [ChannelOnOffControl::default(); 16];
        apply_dc(&mut outputs, 0, DcDrive::Forward(50));
        assert_eq!(outputs[8].off, 2047);
        assert!(outputs[10].full_on);
        assert!(!outputs[9].full_on);
    }

    #[test]
    fn microstep_midpoint_equally() {
        let mut outputs = [ChannelOnOffControl::default(); 16];
        apply_stepper(&mut outputs, 0, StepperState { phase: 4, microsteps: Microsteps::Eight, duty_percent: 100 });
        assert_eq!(outputs[8].off, 2896);
        assert_eq!(outputs[13].off, 2896);
    }

    #[test]
    fn pending_port_commit() {
        // Two commits pending at drain time produce the exact write sequence
        // of one combined batch, and each port is acknowledged individually.
        let writes = Rc::new(RefCell::new(Vec::new()));
        let wing = Box::leak(Box::new(block_on(Wing::connect(RecordingI2c(writes.clone()))).unwrap()));
        let (mut runner, [mut port_a, mut port_b]) = wing.into_steppers();
        let prefix = writes.borrow().len();
        let (first_ack, second_ack, ()) =
            block_on(join3(port_a.hold(Duty::percent(100)), port_b.hold(Duty::percent(100)), runner.service()));
        assert_eq!(first_ack, Ok(()));
        assert_eq!(second_ack, Ok(()));
        let coalesced: Vec<Vec<u8>> = writes.borrow()[prefix..].to_vec();
        assert_ne!(coalesced, Vec::<Vec<u8>>::new());

        let expected = Rc::new(RefCell::new(Vec::new()));
        let mut boundary = block_on(Boundary::connect(RecordingI2c(expected.clone()), TRANSACTION_TIMEOUT)).unwrap();
        let expected_prefix = expected.borrow().len();
        let mut batch = Batch::new();
        batch.hold(StepperSlot::One, Duty::percent(100)).unwrap();
        batch.hold(StepperSlot::Two, Duty::percent(100)).unwrap();
        block_on(boundary.commit(batch)).unwrap();
        assert_eq!(coalesced, expected.borrow()[expected_prefix..].to_vec());
    }

    #[test]
    fn single_port_transactions() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let wing = Box::leak(Box::new(block_on(Wing::connect(RecordingI2c(writes.clone()))).unwrap()));
        let (mut runner, [mut port_a, _port_b]) = wing.into_steppers();
        let (mode_ack, ()) = block_on(join(port_a.set_mode(Microsteps::Four), runner.service()));
        mode_ack.unwrap();
        let (hold_ack, ()) = block_on(join(port_a.hold(Duty::percent(80)), runner.service()));
        hold_ack.unwrap();

        // One full electrical cycle at Microsteps::Four is 16 microsteps.
        let transactions_per_step: Vec<usize> = (0..16u32)
            .map(|_| {
                let before = writes.borrow().len();
                let (step_ack, ()) = block_on(join(port_a.step(Direction::Positive), runner.service()));
                step_ack.unwrap();
                writes.borrow().len() - before
            })
            .collect();

        assert_eq!(transactions_per_step, [2, 2, 2, 3, 2, 2, 2, 3, 2, 2, 2, 3, 2, 2, 2, 3]);
    }

    #[derive(Clone)]
    struct LockCountingI2c {
        writes: Rc<RefCell<Vec<Vec<u8>>>>,
        acquisitions: Rc<RefCell<usize>>,
    }

    impl ErrorType for LockCountingI2c {
        type Error = Infallible;
    }

    impl I2c for LockCountingI2c {
        fn transaction(
            &mut self, _address: u8, operations: &mut [Operation<'_>],
        ) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            *self.acquisitions.borrow_mut() += 1;
            for operation in operations.iter_mut() {
                if let Operation::Write(bytes) = operation {
                    self.writes.borrow_mut().push(bytes.to_vec());
                }
            }
            core::future::ready(Ok(()))
        }
    }

    impl I2cBatch for LockCountingI2c {
        fn write_batch(&mut self, _address: u8, writes: &[&[u8]]) -> impl core::future::Future<Output = Result<(), Self::Error>> {
            *self.acquisitions.borrow_mut() += 1;
            for write in writes {
                self.writes.borrow_mut().push(write.to_vec());
            }
            core::future::ready(Ok(()))
        }
    }

    #[test]
    fn steps_commit_acquisition() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let acquisitions = Rc::new(RefCell::new(0));
        let i2c = LockCountingI2c { writes: writes.clone(), acquisitions: acquisitions.clone() };
        let wing = Box::leak(Box::new(block_on(Wing::connect(i2c)).unwrap()));
        let (mut runner, [mut port_a, _port_b]) = wing.into_steppers();
        let (mode_ack, ()) = block_on(join(port_a.set_mode(Microsteps::Four), runner.service()));
        mode_ack.unwrap();
        let (hold_ack, ()) = block_on(join(port_a.hold(Duty::percent(80)), runner.service()));
        hold_ack.unwrap();

        // Measure one step in isolation, past the mode/hold setup above.
        acquisitions.replace(0);
        let write_count_before = writes.borrow().len();
        let (step_ack, ()) = block_on(join(port_a.step(Direction::Positive), runner.service()));
        step_ack.unwrap();

        assert_eq!(*acquisitions.borrow(), 1);
        assert!(writes.borrow().len() - write_count_before >= 2);
    }
}
