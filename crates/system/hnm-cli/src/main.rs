/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Command-line adapter for Helios Node Mesh.

/* === Modules === */

mod boot;
mod commands;
mod node_action;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::process::ExitCode;

// Third Party Imports
use clap::Parser;

// Local Imports
use crate::commands::Cli;
use hnm_cli::{bootsel, error};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> error::Result<()> {
    boot::initialize(cli.env_file.as_deref())?;

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(cli.run())
}

/* === Tests === */
