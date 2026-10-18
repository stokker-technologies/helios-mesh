/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// The gateway role: mesh brokerage with no role-owned request handling.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use hnm::Node;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[hnm::node]
pub struct Gateway;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[hnm::node]
impl Node for Gateway {
    type Config = ();

    const NODE_TYPE: hnm::NodeType = hnm::NodeType::Gateway;
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
