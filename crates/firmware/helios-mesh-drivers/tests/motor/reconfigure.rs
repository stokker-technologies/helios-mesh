/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Verify Reconfiguration Ownership While Motor Operations Run.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use core::cell::RefCell;
use core::convert::Infallible;
use std::boxed::Box;
use std::rc::Rc;
use std::vec::Vec;
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

// Third Party Imports
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Instant, MockDriver, Timer};

// Local Imports
use helios_mesh_drivers::motor::*;
use helios_mesh_protocol::{
    motion::{Mechanics, Microsteps, Power},
    node::OperationId,
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone)]
struct RecordingDriver(Rc<RefCell<Vec<Microsteps>>>);

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl StepperDriver for RecordingDriver {
    type Error = Infallible;

    fn set_mode(&mut self, microsteps: Microsteps) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        self.0.borrow_mut().push(microsteps);
        core::future::ready(Ok(()))
    }

    fn step(&mut self, _direction: Direction) -> impl core::future::Future<Output = Result<StepTiming, Self::Error>> {
        core::future::ready(Ok(StepTiming::default()))
    }

    fn hold(&mut self, _duty: Duty) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        core::future::ready(Ok(()))
    }

    fn release(&mut self) -> impl core::future::Future<Output = Result<(), Self::Error>> {
        core::future::ready(Ok(()))
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn config(microsteps: Microsteps) -> Config {
    Config { mechanics: Mechanics { full_steps: 8, microsteps, reversed: false }, power: Power { moving: 80, holding: 20 } }
}

/* === Tests === */

#[test]
fn reconfigure_applies_operations() {
    let commands: &'static Commands = Box::leak(Box::new(Commands::new()));
    let completions: &'static Completions = Box::leak(Box::new(Completions::new()));
    let modes = Rc::new(RefCell::new(Vec::new()));
    let mut stepper = Stepper::new(RecordingDriver(modes.clone()), config(Microsteps::Four), commands.receiver(), completions.sender());
    let handle = Handle::new(commands);

    let script = async {
        // An idle reconfigure applies before the next operation.
        handle.try_reconfigure(config(Microsteps::Eight)).unwrap();
        handle.try_set_home(OperationId(1)).unwrap();
        let homed = completions.receive().await;
        assert_eq!(homed.outcome, Outcome::Homed);

        // A reconfigure arriving mid-motion is ignored.
        handle
            .try_move(Move {
                operation: OperationId(2),
                target_microdegrees: 90_000_000,
                maximum_microdegrees_per_second: 3_600_000_000,
                acceleration_microdegrees_per_second_squared: 3_600_000_000,
                start: Instant::now() + Duration::from_millis(50),
            })
            .unwrap();
        Timer::after(Duration::from_millis(5)).await;
        handle.try_reconfigure(config(Microsteps::Four)).unwrap();
        let reached = completions.receive().await;
        assert!(matches!(reached.outcome, Outcome::Reached { .. }));

        // The ignored definition never resynchronizes the mode.
        handle.try_set_home(OperationId(3)).unwrap();
        completions.receive().await;
    };
    let mut running = pin!(select(stepper.run(), script));
    loop {
        match running.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(Either::Second(())) => break,
            Poll::Ready(Either::First(never)) => match never {},
            Poll::Pending => MockDriver::get().advance(Duration::from_micros(20)),
        }
    }

    assert_eq!(modes.borrow().as_slice(), &[Microsteps::Eight]);
}
