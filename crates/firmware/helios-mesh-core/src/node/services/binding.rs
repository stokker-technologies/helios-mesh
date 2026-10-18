/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Chassis-owned single write-binding latch with idle expiry.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use helios_mesh_protocol::node::{NodeId, SessionId};

/* === Constants === */

pub const BINDING_TIMEOUT_MS: u64 = 30_000;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Holder {
    gateway: NodeId,
    session: SessionId,
    last_heard_ms: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Binding {
    holder: Option<Holder>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Binding {
    #[must_use]
    pub const fn new() -> Self {
        Self { holder: None }
    }

    pub fn admit(&mut self, peer: NodeId, session: SessionId, write: bool, now_ms: u64) -> Result<(), NodeId> {
        self.expire(now_ms);
        match self.holder {
            Some(holder) if holder.gateway == peer && holder.session == session => {
                self.holder = Some(Holder { last_heard_ms: now_ms, ..holder });
                Ok(())
            }
            Some(holder) if holder.gateway == peer => {
                if write {
                    self.bind(peer, session, now_ms);
                }
                Ok(())
            }
            Some(holder) => {
                if write {
                    Err(holder.gateway)
                } else {
                    Ok(())
                }
            }
            None => {
                if write {
                    self.bind(peer, session, now_ms);
                }
                Ok(())
            }
        }
    }

    pub fn is_bound(&mut self, now_ms: u64) -> bool {
        self.expire(now_ms);
        self.holder.is_some()
    }

    pub fn expire(&mut self, now_ms: u64) -> bool {
        let expired = self.holder.is_some_and(|holder| now_ms.saturating_sub(holder.last_heard_ms) >= BINDING_TIMEOUT_MS);
        if expired {
            self.holder = None;
        }
        expired
    }

    fn bind(&mut self, gateway: NodeId, session: SessionId, now_ms: u64) {
        self.holder = Some(Holder { gateway, session, last_heard_ms: now_ms });
    }
}

impl Default for Binding {
    fn default() -> Self {
        Self::new()
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    const GATEWAY: NodeId = NodeId(273_282);
    const OTHER_GATEWAY: NodeId = NodeId(566_114);
    const SESSION: SessionId = SessionId(1);
    const LATER_SESSION: SessionId = SessionId(2);

    #[test]
    fn first_radio_node() {
        let mut binding = Binding::new();
        assert!(!binding.is_bound(0));
        assert_eq!(binding.admit(GATEWAY, SESSION, true, 0), Ok(()));
        assert!(binding.is_bound(0));
    }

    #[test]
    fn write_second_first() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        assert_eq!(binding.admit(OTHER_GATEWAY, SESSION, true, 100), Err(GATEWAY));
        // The rejection leaves the original binding untouched.
        assert_eq!(binding.admit(GATEWAY, SESSION, true, 200), Ok(()));
    }

    #[test]
    fn reads_any_bound() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        assert_eq!(binding.admit(OTHER_GATEWAY, SESSION, false, 100), Ok(()));
        // A read from a non-bound gateway must not steal or clear the
        // binding.
        assert_eq!(binding.admit(OTHER_GATEWAY, SESSION, true, 200), Err(GATEWAY));
    }

    #[test]
    fn binding_idle_binds() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        assert!(binding.is_bound(BINDING_TIMEOUT_MS - 1));
        assert_eq!(
            binding.admit(OTHER_GATEWAY, SESSION, true, BINDING_TIMEOUT_MS - 1),
            Err(GATEWAY),
            "one millisecond short of the timeout the original binding still holds"
        );
        assert_eq!(
            binding.admit(OTHER_GATEWAY, SESSION, true, BINDING_TIMEOUT_MS),
            Ok(()),
            "exactly at the timeout the idle binding lapses and the next write binds"
        );
        assert!(binding.is_bound(BINDING_TIMEOUT_MS));
    }

    #[test]
    fn any_request_clock() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        // A read just before the deadline refreshes it, same as a write.
        binding.admit(GATEWAY, SESSION, false, BINDING_TIMEOUT_MS - 1).unwrap();
        assert!(binding.is_bound(2 * BINDING_TIMEOUT_MS - 2));
    }

    #[test]
    fn different_session_write() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        assert_eq!(binding.admit(GATEWAY, LATER_SESSION, true, 100), Ok(()));
        // SessionId is an opaque random identity, not an ordered generation.
        // The latch can only implement the operational reboot premise as
        // same-gateway/different-session immediate takeover; it keeps no
        // session history and makes no old-versus-new ordering claim.
        assert_eq!(
            binding.admit(OTHER_GATEWAY, SESSION, true, 200),
            Err(GATEWAY),
            "the same gateway id remains the holder after its session changes"
        );
    }

    #[test]
    fn read_bound_over() {
        let mut binding = Binding::new();
        binding.admit(GATEWAY, SESSION, true, 0).unwrap();
        assert_eq!(binding.admit(GATEWAY, LATER_SESSION, false, 100), Ok(()));
        // Binding is implicit on writes only: a mere read from a new
        // session must not have taken the binding over.
        assert_eq!(binding.admit(OTHER_GATEWAY, SESSION, true, 200), Err(GATEWAY));
    }
}
