/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Mesh-brokerage capability: gateway semantics beyond plain forwarding.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::node::{NodeId, NodeSummary};

/* === Constants === */

pub const NODE_PAGE_SIZE: usize = 4;

/* === Macros === */

wrap_requests!(Gateway: Nodes);

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Nodes {
    pub after: Option<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodePage {
    pub nodes: heapless::Vec<NodeSummary, NODE_PAGE_SIZE>,

    pub next_after: Option<NodeId>,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Request {
    Nodes(Nodes),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Nodes(NodePage),
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
