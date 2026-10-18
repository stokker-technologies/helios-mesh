/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `gpio-pulse`: schedule a high pulse on the node's local timer.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::time::Duration;

// Third Party Imports
use clap::Args as ClapArgs;

// Local Imports
use crate::error::Result;
use helios_mesh::{NodeHandle, node::AuxGpioId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[arg(value_parser = clap::value_parser!(u8).range(1..=4))]
    gpio: u8,

    #[arg(value_parser = clap::value_parser!(u32).range(1..=60_000))]
    duration_ms: u32,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, args: Args) -> Result<()> {
    helios_mesh::SidecarNode::from_handle(node.clone())
        .await?
        .pulse_gpio(AuxGpioId(args.gpio), Duration::from_millis(u64::from(args.duration_ms)))
        .await?;
    println!("scheduled node {:06} gpio={} pulse_ms={}", node.id().0, args.gpio, args.duration_ms);
    Ok(())
}

/* === Private Functions === */

/* === Tests === */
