/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `nodes`: list nodes reachable through the addressed gateway.

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
    let gateway = hnm::GatewayNode::from_handle(node.clone()).await?;
    let nodes = gateway.scan().await?;
    if nodes.is_empty() {
        println!("no reachable nodes");
        return Ok(());
    }
    for summary in &nodes {
        node_action::print_info(&summary.info);
        println!("  {}", last_seen_line(summary.silent_for_s));
    }
    Ok(())
}

/* === Private Functions === */

fn last_seen_line(silent_for_s: u16) -> String {
    format!("last_seen={silent_for_s}s ago")
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_seen_ago() {
        assert_eq!(last_seen_line(0), "last_seen=0s ago");
        assert_eq!(last_seen_line(14), "last_seen=14s ago");
    }
}
