/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise RFM95 Reset, Both Interrupt Lines And Cancelled Wait Recovery.

/* === Modules === */

mod driver;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    cell::{Cell, RefCell},
    future::{Future, poll_fn},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

// Third Party Imports
use embassy_futures::block_on;
use embedded_hal::digital::{Error as DigitalError, ErrorKind, ErrorType, OutputPin};
use embedded_hal_async::{delay::DelayNs, digital::Wait};
use lora_phy::{mod_params::RadioError, mod_traits::InterfaceVariant};

// Local Imports
use helios_mesh_board_feather_rp2040::radio::Interface;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Default)]
struct Signal {
    high: Cell<bool>,
    failed: Cell<bool>,
    cancelled: Cell<usize>,
    waker: RefCell<Option<Waker>>,
}

struct Input(Rc<Signal>);

struct Reset(Rc<RefCell<Vec<bool>>>);

#[derive(Default)]
struct Delay(Vec<u32>);

struct Waiting(Option<Rc<Signal>>);

/* === Enums === */

#[derive(Debug)]
enum PinError {
    Fault,
}

/* === Traits === */

/* === Implementations === */

impl DigitalError for PinError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl ErrorType for Input {
    type Error = PinError;
}

impl ErrorType for Reset {
    type Error = PinError;
}

impl OutputPin for Reset {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.borrow_mut().push(false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.borrow_mut().push(true);
        Ok(())
    }
}

impl DelayNs for Delay {
    fn delay_ns(&mut self, nanoseconds: u32) -> impl Future<Output = ()> {
        self.0.push(nanoseconds);
        std::future::ready(())
    }
}

impl Wait for Input {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        let mut waiting = Waiting(Some(self.0.clone()));
        let result = poll_fn(|context| {
            if self.0.failed.get() {
                return Poll::Ready(Err(PinError::Fault));
            }
            if self.0.high.get() {
                return Poll::Ready(Ok(()));
            }
            *self.0.waker.borrow_mut() = Some(context.waker().clone());
            Poll::Pending
        })
        .await;
        waiting.0.take();
        result
    }

    fn wait_for_low(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Err(PinError::Fault))
    }

    fn wait_for_rising_edge(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Err(PinError::Fault))
    }

    fn wait_for_falling_edge(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Err(PinError::Fault))
    }

    fn wait_for_any_edge(&mut self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Err(PinError::Fault))
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        if let Some(signal) = self.0.take() {
            signal.cancelled.set(signal.cancelled.get() + 1);
        }
    }
}

impl Signal {
    fn trigger(&self) {
        self.high.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[test]
fn reset_sequence() {
    let reset = Rc::new(RefCell::new(Vec::new()));
    let dio0 = Rc::new(Signal::default());
    let dio1 = Rc::new(Signal::default());
    let mut interface = Interface::new(Reset(reset.clone()), Input(dio0), Input(dio1));
    let mut delay = Delay::default();
    block_on(interface.reset(&mut delay)).unwrap();

    assert_eq!(*reset.borrow(), [false, true]);
    assert_eq!(delay.0, [10_000_000, 10_000_000, 10_000_000]);
    block_on(interface.wait_on_busy()).unwrap();
    block_on(interface.enable_rf_switch_rx()).unwrap();
    block_on(interface.enable_rf_switch_tx()).unwrap();
    block_on(interface.disable_rf_switch()).unwrap();
}

#[test]
fn interrupt_sources() {
    for source in 0..2 {
        let signals = [Rc::new(Signal::default()), Rc::new(Signal::default())];
        let reset = Rc::new(RefCell::new(Vec::new()));
        let mut interface = Interface::new(Reset(reset), Input(signals[0].clone()), Input(signals[1].clone()));
        let mut context = Context::from_waker(Waker::noop());
        {
            let mut waiting = pin!(interface.await_irq());
            assert!(waiting.as_mut().poll(&mut context).is_pending());
            signals[source].trigger();
            assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
        }
        assert_eq!(signals[1 - source].cancelled.get(), 1);

        signals[source].high.set(false);
        {
            let mut waiting = pin!(interface.await_irq());
            assert!(waiting.as_mut().poll(&mut context).is_pending());
        }
        signals[1 - source].trigger();
        block_on(interface.await_irq()).unwrap();
        signals[1 - source].high.set(false);
        signals[source].failed.set(true);
        assert!(matches!(block_on(interface.await_irq()), Err(RadioError::Irq)));
    }
}
