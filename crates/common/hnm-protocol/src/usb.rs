/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared USB identity contract for HNM nodes.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::node::MAX_NODE_LABEL;

/* === Constants === */

pub const VENDOR_ID: u16 = 0xC0DE;
pub const PRODUCT_ID: u16 = 0x484E;
pub const MANUFACTURER: &str = "Helios";
pub const PRODUCT_PREFIX: &str = "Helios Node Mesh";
pub const MAX_PRODUCT: usize = PRODUCT_PREFIX.len() + 1 + MAX_NODE_LABEL;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

#[must_use]
pub fn product(label: &str) -> heapless::String<MAX_PRODUCT> {
    let mut product = heapless::String::new();
    // The capacity always holds the prefix and separator.
    let _ = product.push_str(PRODUCT_PREFIX);
    let _ = product.push(' ');
    for character in label.chars() {
        if product.push(character).is_err() {
            break;
        }
    }
    product
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_string_name() {
        assert_eq!(product("Rotator").as_str(), "Helios Node Mesh Rotator");
    }

    #[test]
    fn oversized_display_truncated() {
        let label = "An Unreasonably Long Display Name";
        let product = product(label);
        assert_eq!(product.len(), MAX_PRODUCT);
        assert!(product.as_str().starts_with("Helios Node Mesh An Unreas"));
    }
}
