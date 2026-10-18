/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Own The RFM95 Reset And Both Interrupt Lines Through The Published Driver Interface.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_futures::select::{Either, select};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::{delay::DelayNs, digital::Wait};
use lora_phy::{mod_params::RadioError, mod_traits::InterfaceVariant};

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Interface<RESET, IRQ> {
    reset: RESET,
    dio0: IRQ,
    dio1: IRQ,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl<RESET, IRQ> Interface<RESET, IRQ> {
    pub const fn new(reset: RESET, dio0: IRQ, dio1: IRQ) -> Self {
        Self { reset, dio0, dio1 }
    }
}

impl<RESET: OutputPin, IRQ: Wait> InterfaceVariant for Interface<RESET, IRQ> {
    async fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), RadioError> {
        delay.delay_ms(10).await;
        self.reset.set_low().map_err(|_| RadioError::Reset)?;
        delay.delay_ms(10).await;
        self.reset.set_high().map_err(|_| RadioError::Reset)?;
        delay.delay_ms(10).await;
        Ok(())
    }

    fn wait_on_busy(&mut self) -> impl core::future::Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }

    async fn await_irq(&mut self) -> Result<(), RadioError> {
        let event = select(self.dio0.wait_for_high(), self.dio1.wait_for_high()).await;
        let result = match event {
            Either::First(result) | Either::Second(result) => result,
        };
        result.map_err(|_| RadioError::Irq)
    }

    fn enable_rf_switch_rx(&mut self) -> impl core::future::Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }

    fn enable_rf_switch_tx(&mut self) -> impl core::future::Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }

    fn disable_rf_switch(&mut self) -> impl core::future::Future<Output = Result<(), RadioError>> {
        core::future::ready(Ok(()))
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
