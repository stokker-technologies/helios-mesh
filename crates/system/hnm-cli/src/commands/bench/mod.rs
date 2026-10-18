/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `bench`: explicit performance probes.

/* === Modules === */

mod dispatcher;
mod rtt;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::{Args as ClapArgs, Subcommand};

// Local Imports
use crate::{boot::Target, error::Result};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

/* === Enums === */

#[derive(Debug, Subcommand)]
enum Command {
    Rtt(rtt::Args),

    Dispatcher(dispatcher::Args),
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(target: &Target, args: Args) -> Result<()> {
    match args.command {
        Command::Rtt(args) => rtt::run(target, args).await,
        Command::Dispatcher(args) => dispatcher::run(args).await,
    }
}

pub(crate) fn percentile(samples: &[u64], percentile: usize) -> u64 {
    let index = samples.len().saturating_mul(percentile).div_ceil(100).saturating_sub(1);
    samples[index.min(samples.len() - 1)]
}

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::percentile;

    #[test]
    fn percentile_uses_rank() {
        let samples = [10, 20, 30, 40, 50];
        assert_eq!(percentile(&samples, 50), 30);
        assert_eq!(percentile(&samples, 95), 50);
    }
}
