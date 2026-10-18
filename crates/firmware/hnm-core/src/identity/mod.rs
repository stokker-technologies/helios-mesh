/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Hardware-derived node identities.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm_protocol::node::{NodeId, SessionId};

/* === Constants === */

const DEVICE_NODE_ID_COUNT: u64 = 900_000;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub node: NodeId,

    pub session: SessionId,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[must_use]
pub fn from_hardware_unique_id(unique_id: &[u8]) -> NodeId {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in b"HNM".iter().chain(unique_id) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    NodeId(NodeId::MIN_DEVICE + u32::try_from(hash % DEVICE_NODE_ID_COUNT).expect("six-digit ID fits in u32"))
}

#[must_use]
pub fn decimal_digits(node_id: NodeId) -> [u8; 6] {
    let mut digits = [b'0'; 6];
    let mut value = node_id.0;
    let mut index = digits.len();
    while index > 0 {
        index -= 1;
        digits[index] += (value % 10) as u8;
        value /= 10;
    }
    digits
}

/* === Private Functions === */

/* === Tests === */
