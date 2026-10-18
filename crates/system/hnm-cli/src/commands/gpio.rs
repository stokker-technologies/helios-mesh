/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `gpio`: configure one shared auxiliary GPIO.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use clap::{Args as ClapArgs, ValueEnum};

// Local Imports
use crate::{error::Result, node_action};
use hnm::{NodeHandle, node::AuxGpioMode};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    gpio: u8,

    mode: Mode,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Mode {
    HighZ,
    High,
    Low,
}

/* === Traits === */

/* === Implementations === */

impl From<Mode> for AuxGpioMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::HighZ => Self::HighImpedance,
            Mode::High => Self::DriveHigh,
            Mode::Low => Self::DriveLow,
        }
    }
}

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, args: Args) -> Result<()> {
    node_action::gpio(node, args.gpio, args.mode.into()).await
}

/* === Private Functions === */

/* === Tests === */
