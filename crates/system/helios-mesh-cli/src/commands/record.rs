/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `record`: continuously save journal records as JSONL until interrupted.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

// Third Party Imports
use clap::Args as ClapArgs;
use serde::Serialize;
use tokio::time::{Duration, sleep};

// Local Imports
use crate::error::Result;
use helios_mesh::{
    NodeHandle,
    journal::{Cursor, Header, Record},
};

/* === Constants === */

const POLL_INTERVAL: Duration = Duration::from_millis(250);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    output: PathBuf,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(node: &NodeHandle, args: Args) -> Result<()> {
    let node_id = node.id();
    let file = create(&args.output)?;
    let mut writer = BufWriter::new(file);
    write_line(&mut writer, &Header::new(node_id))?;
    writer.flush()?;
    println!("recording node {:06} journal to {}; press Ctrl-C to stop", node_id.0, args.output.display());

    let mut cursor = Cursor::new();
    let mut position = 1_u64;
    let interrupted = tokio::signal::ctrl_c();
    tokio::pin!(interrupted);
    loop {
        let entries = cursor.next(node).await?;
        for entry in &entries {
            write_line(&mut writer, &Record::new(position, node_id, entry))?;
            position = position.saturating_add(1);
        }
        writer.flush()?;

        if !cursor.is_caught_up() {
            continue;
        }

        tokio::select! {
            result = &mut interrupted => {
                result?;
                break;
            }
            () = sleep(POLL_INTERVAL) => {}
        }
    }

    println!("saved {} records to {}", position - 1, args.output.display());
    Ok(())
}

/* === Private Functions === */

fn create(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}

fn write_line(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    Ok(())
}

/* === Tests === */
