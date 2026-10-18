/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `move`: move one advertised rotational channel, by absolute target or

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::Args as ClapArgs;

// Local Imports
use crate::{error::Result, node_action};
use helios_mesh::NodeHandle;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    channel: u8,

    #[arg(allow_hyphen_values = true)]
    degrees: f64,

    #[arg(long)]
    relative: bool,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, args: Args) -> Result<()> {
    node_action::move_to(node, args.channel, args.degrees, args.relative).await
}

/* === Private Functions === */

/* === Tests === */
