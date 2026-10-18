/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise Motor Wing Transactions And Recovery With A Controlled Clock.

/* === Modules === */

mod reconfigure;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    cell::RefCell,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

// Third Party Imports
use embassy_futures::{
    block_on,
    join::join,
    select::{Either, select},
};
use embassy_time::{Duration, MockDriver};
use embedded_hal::i2c::{ErrorKind, ErrorType, Operation};
use embedded_hal_async::i2c::I2c;

// Local Imports
use hnm_drivers::{
    bus::i2c::I2cBatch,
    motor::{
        Direction, Duty, StepperDriver,
        wing::{OutputUnavailable, Wing},
    },
};
use hnm_protocol::motion::Microsteps;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct State {
    registers: [u8; 256],
    pending: bool,
    fail_next: bool,
    acquisitions: usize,
    writes: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct Bus(Rc<RefCell<State>>);

#[derive(Debug)]
struct BusFault;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl State {
    fn new(pending: bool) -> Self {
        Self { registers: [0; 256], pending, fail_next: false, acquisitions: 0, writes: Vec::new() }
    }

    fn write(&mut self, bytes: &[u8]) {
        let Some((&address, payload)) = bytes.split_first() else { return };
        for (offset, value) in payload.iter().enumerate() {
            self.registers[usize::from(address) + offset] = *value;
        }
        self.writes.push(bytes.to_vec());
    }
}

impl ErrorType for Bus {
    type Error = BusFault;
}

impl embedded_hal::i2c::Error for BusFault {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl I2c for Bus {
    fn transaction(&mut self, address: u8, operations: &mut [Operation<'_>]) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::poll_fn(move |_| {
            assert_eq!(address, 0x60);
            let mut state = self.0.borrow_mut();
            if state.pending {
                return Poll::Pending;
            }
            if core::mem::take(&mut state.fail_next) {
                return Poll::Ready(Err(BusFault));
            }
            state.acquisitions += 1;
            let mut pointer = 0;
            for operation in operations.iter_mut() {
                match operation {
                    Operation::Write(bytes) => {
                        state.write(bytes);
                        pointer = usize::from(bytes[0]);
                    }
                    Operation::Read(bytes) => {
                        for byte in bytes.iter_mut() {
                            *byte = state.registers[pointer];
                            pointer += 1;
                        }
                    }
                }
            }
            Poll::Ready(Ok(()))
        })
    }
}

impl I2cBatch for Bus {
    fn write_batch(&mut self, address: u8, writes: &[&[u8]]) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::poll_fn(move |_| {
            assert_eq!(address, 0x60);
            let mut state = self.0.borrow_mut();
            if state.pending {
                return Poll::Pending;
            }
            if core::mem::take(&mut state.fail_next) {
                return Poll::Ready(Err(BusFault));
            }
            state.acquisitions += 1;
            for bytes in writes {
                state.write(bytes);
            }
            Poll::Ready(Ok(()))
        })
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[test]
fn initialization_preserves_cause() {
    let state = Rc::new(RefCell::new(State::new(false)));
    state.borrow_mut().fail_next = true;
    let result = block_on(Wing::connect(Bus(state.clone())));
    assert!(matches!(result, Err(hnm_drivers::Error::Device(pwm_pca9685::Error::I2C(BusFault)))));

    let wing = block_on(Wing::connect(Bus(state.clone())));
    assert!(wing.is_ok());
    assert_eq!(state.borrow().registers[0xFE], 3);
}

#[test]
fn initialization_timeout() {
    let state = Rc::new(RefCell::new(State::new(true)));
    let mut connecting = pin!(Wing::connect(Bus(state.clone())));
    let mut context = Context::from_waker(Waker::noop());
    assert!(connecting.as_mut().poll(&mut context).is_pending());
    MockDriver::get().advance(Duration::from_millis(249));
    assert!(connecting.as_mut().poll(&mut context).is_pending());
    MockDriver::get().advance(Duration::from_millis(1));
    assert!(matches!(connecting.as_mut().poll(&mut context), Poll::Ready(Err(hnm_drivers::Error::Timeout))));
    assert_eq!(state.borrow().acquisitions, 0);
}

#[test]
fn paired_steps_recover() {
    let state = Rc::new(RefCell::new(State::new(false)));
    let wing = Box::leak(Box::new(block_on(Wing::connect(Bus(state.clone()))).unwrap()));
    let (mut runner, [mut first, mut second]) = wing.into_steppers();
    let Either::First((a, b)) = block_on(select(join(first.set_mode(Microsteps::Four), second.set_mode(Microsteps::Four)), runner.run()));
    a.unwrap();
    b.unwrap();
    let Either::First((a, b)) = block_on(select(join(first.hold(Duty::percent(80)), second.hold(Duty::percent(80))), runner.run()));
    a.unwrap();
    b.unwrap();

    state.borrow_mut().acquisitions = 0;
    state.borrow_mut().writes.clear();
    let mut context = Context::from_waker(Waker::noop());
    {
        let mut a = pin!(first.step(Direction::Positive));
        assert!(a.as_mut().poll(&mut context).is_pending());
        MockDriver::get().advance(Duration::from_millis(5));
        let mut b = pin!(second.step(Direction::Negative));
        assert!(b.as_mut().poll(&mut context).is_pending());
        let mut worker = pin!(runner.run());
        assert!(worker.as_mut().poll(&mut context).is_pending());
        let timing = block_on(a).unwrap();
        assert_eq!(timing.queued_us, 5_000);
        assert_eq!(timing.transaction_us, 0);
        assert_eq!(block_on(b).unwrap().queued_us, 0);
    }
    assert_eq!(state.borrow().acquisitions, 1);
    assert_ne!(state.borrow().writes, Vec::<Vec<u8>>::new());

    state.borrow_mut().pending = true;
    {
        let mut step = pin!(first.step(Direction::Positive));
        assert!(step.as_mut().poll(&mut context).is_pending());
        let mut worker = pin!(runner.run());
        assert!(worker.as_mut().poll(&mut context).is_pending());
        MockDriver::get().advance(Duration::from_millis(250));
        assert!(worker.as_mut().poll(&mut context).is_pending());
        assert_eq!(block_on(step), Err(OutputUnavailable));
    }
    state.borrow_mut().pending = false;
    let Either::First(result) = block_on(select(first.step(Direction::Positive), runner.run()));
    result.unwrap();
}
