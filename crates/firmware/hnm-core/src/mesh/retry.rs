/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Bounded constant retry timing for gateway-owned LoRa exchanges.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_time::Duration;

// Local Imports

/* === Constants === */

pub const INTERACTIVE: Policy =
    Policy { attempts: 3, response_timeout: Duration::from_millis(200), retry_interval: Duration::from_millis(20) };
pub const BACKGROUND: Policy = Policy { attempts: 1, response_timeout: Duration::from_millis(120), retry_interval: Duration::MIN };

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Policy {
    pub attempts: u8,

    pub response_timeout: Duration,

    pub retry_interval: Duration,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Policy {
    #[must_use]
    pub const fn delay(self) -> Duration {
        self.retry_interval
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_uses_interval() {
        assert_eq!(INTERACTIVE.delay(), Duration::from_millis(20));
    }

    #[test]
    fn background_work_retries() {
        assert_eq!(BACKGROUND.attempts, 1);
        assert_eq!(BACKGROUND.response_timeout, Duration::from_millis(120));
    }
}
