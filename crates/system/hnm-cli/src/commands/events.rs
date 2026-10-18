/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `events`: read the node's retained diagnostic journal.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::Args as ClapArgs;

// Local Imports
use crate::{error::Result, node_action};
use hnm::NodeHandle;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, _args: Args) -> Result<()> {
    node_action::events(node).await
}

/* === Private Functions === */

/* === Tests === */
