/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// One command tree with global target-resolution flags.

/* === Modules === */

mod bench;
mod describe;
mod diagnostics;
mod events;
mod flash;
mod gpio;
mod gpio_pulse;
mod identify;
mod move_to;
mod nodes;
mod record;
mod restart;
mod status;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::{Parser, Subcommand};

// Local Imports
use crate::{
    boot::Target,
    error::{Error, Result},
};
use hnm::node::NodeId;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, Parser)]
#[command(name = "hnm", version, about = "Control a Helios Node Mesh")]
pub(crate) struct Cli {
    #[arg(long, global = true)]
    pub(crate) env_file: Option<std::path::PathBuf>,

    #[arg(long, global = true, conflicts_with = "port")]
    device: Option<String>,

    #[arg(long, global = true, conflicts_with = "device")]
    port: Option<String>,

    #[arg(long, global = true)]
    node: Option<String>,

    #[command(subcommand)]
    command: Command,
}

/* === Enums === */

#[derive(Debug, Subcommand)]
enum Command {
    Status(status::Args),

    Describe(describe::Args),

    Diagnostics(diagnostics::Args),

    Events(events::Args),

    Record(record::Args),

    Identify(identify::Args),

    Gpio(gpio::Args),

    GpioPulse(gpio_pulse::Args),

    Restart(restart::Args),

    Flash(flash::Args),

    Move(move_to::Args),

    Nodes(nodes::Args),

    Bench(bench::Args),
}

/* === Traits === */

/* === Implementations === */

impl Cli {
    pub(crate) async fn run(self) -> Result<()> {
        let device = self.device.as_deref().map(parse_node_id).transpose()?;
        let node = self.node.as_deref().map(parse_node_id).transpose()?;
        let target = Target::new(device, self.port, node);
        if let Command::Bench(args) = self.command {
            return bench::run(&target, args).await;
        }
        if let Command::Flash(args) = self.command {
            return flash::run(&target, args).await;
        }
        if let Command::Nodes(args) = self.command {
            // `nodes` inherently needs mesh brokerage: connect through the
            // coordinator-checked door, not the generic `target.connect()`
            // below, which never probes an ambiguous bus for the
            // coordinator (see `Target::connect_gateway`'s doc comment).
            let node = target.connect_gateway().await?;
            return nodes::run(&node, args).await;
        }
        let node = target.connect().await?;
        match self.command {
            Command::Status(args) => status::run(&node, args).await,
            Command::Describe(args) => describe::run(&node, args).await,
            Command::Diagnostics(args) => diagnostics::run(&node, args).await,
            Command::Events(args) => events::run(&node, args).await,
            Command::Record(args) => record::run(&node, args).await,
            Command::Identify(args) => identify::run(&node, args).await,
            Command::Gpio(args) => gpio::run(&node, args).await,
            Command::GpioPulse(args) => gpio_pulse::run(&node, args).await,
            Command::Restart(args) => restart::run(&node, args).await,
            Command::Move(args) => move_to::run(&node, args).await,
            Command::Nodes(_) | Command::Bench(_) | Command::Flash(_) => {
                unreachable!("nodes, bench, and flash dispatch before target connection")
            }
        }
    }
}

/* === Public Functions === */

pub(crate) fn parse_node_id(value: &str) -> Result<NodeId> {
    let id = value.parse::<u32>().map(NodeId).map_err(|_| Error::InvalidNodeId(value.into()))?;
    if value.len() != 6 || !id.is_device() {
        return Err(Error::InvalidNodeId(value.into()));
    }
    Ok(id)
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_identifiers() {
        assert_eq!(parse_node_id("273282").unwrap(), NodeId(273_282));
        assert!(parse_node_id("27328").is_err());
        assert!(parse_node_id("099999").is_err());
        assert!(parse_node_id("27328x").is_err());
    }
}
