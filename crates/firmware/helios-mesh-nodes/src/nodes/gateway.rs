/* ***
 *
 * Helios Mesh
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
use helios_mesh::Node;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[helios_mesh::node]
pub struct Gateway;

/* === Enums === */

/* === Traits === */

/* === Implementations === */

#[helios_mesh::node]
impl Node for Gateway {
    type Config = ();

    const NODE_TYPE: helios_mesh::NodeType = helios_mesh::NodeType::Gateway;
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
