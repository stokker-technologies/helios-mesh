/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `restart`: restart the addressed node, optionally into a bootloader.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::Args as ClapArgs;

// Local Imports
use crate::{error::Result, node_action};
use helios_mesh::{NodeHandle, node::Bootloader};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[arg(long)]
    bootloader: bool,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, args: Args) -> Result<()> {
    node_action::restart(node, args.bootloader.then_some(Bootloader::Usb)).await
}

/* === Private Functions === */

/* === Tests === */
