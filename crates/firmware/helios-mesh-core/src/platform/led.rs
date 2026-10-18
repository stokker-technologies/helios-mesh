/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Status LED signal vocabulary shared by the chassis and board LED runners.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

// Local Imports

/* === Constants === */

const CAPACITY: usize = 8;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub struct Signals {
    events: Channel<CriticalSectionRawMutex, Message, CAPACITY>,
}

#[derive(Clone, Copy)]
pub struct Handle {
    signals: &'static Signals,
}

pub struct Listener {
    signals: &'static Signals,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Message {
    Communicating,

    Identify,
}

/* === Traits === */

/* === Implementations === */

impl Signals {
    #[must_use]
    pub const fn new() -> Self {
        Self { events: Channel::new() }
    }

    #[must_use]
    pub fn split(&'static self) -> (Handle, Listener) {
        (Handle { signals: self }, Listener { signals: self })
    }
}

impl Default for Signals {
    fn default() -> Self {
        Self::new()
    }
}

impl Handle {
    pub fn communicating(self) {
        let _ = self.signals.events.try_send(Message::Communicating);
    }

    #[must_use]
    pub fn identify(self) -> bool {
        self.signals.events.try_send(Message::Identify).is_ok()
    }
}

impl Listener {
    pub async fn next(&mut self) -> Message {
        self.signals.events.receive().await
    }

    pub fn try_next(&mut self) -> Option<Message> {
        self.signals.events.try_receive().ok()
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::{Message, Signals};

    #[test]
    fn signals_delivered_admission() {
        static SIGNALS: Signals = Signals::new();
        let (handle, mut listener) = SIGNALS.split();

        handle.communicating();
        assert!(handle.identify());

        assert_eq!(listener.try_next(), Some(Message::Communicating));
        assert_eq!(listener.try_next(), Some(Message::Identify));
        assert_eq!(listener.try_next(), None);
    }
}
