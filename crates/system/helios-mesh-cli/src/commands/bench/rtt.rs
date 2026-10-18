/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `bench rtt`: measure status-request round trips to the addressed node.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::time::{Duration, Instant};

// Third Party Imports
use clap::Args as ClapArgs;

// Local Imports
use super::percentile;
use crate::{boot::Target, error::Result};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[arg(long, default_value_t = 60)]
    seconds: u64,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(target: &Target, args: Args) -> Result<()> {
    let node = target.connect().await?;
    let duration = Duration::from_secs(args.seconds);
    let benchmark_started = Instant::now();
    let mut round_trips = Vec::new();
    let mut failed = 0_u64;

    while benchmark_started.elapsed() < duration {
        let request_started = Instant::now();
        match node.status().await {
            Ok(_) => round_trips.push(u64::try_from(request_started.elapsed().as_millis()).unwrap_or(u64::MAX)),
            Err(_) => failed = failed.saturating_add(1),
        }
    }

    round_trips.sort_unstable();
    let successful = round_trips.len();
    let elapsed_ms = benchmark_started.elapsed().as_millis();
    if successful == 0 {
        println!("duration_ms={elapsed_ms} successful=0 failed={failed} throughput_hz=0.00");
        return Ok(());
    }

    let total_ms: u128 = round_trips.iter().copied().map(u128::from).sum();
    let successful_count = u128::try_from(successful).unwrap_or(u128::MAX);
    let mean_tenths = total_ms.saturating_mul(10) / successful_count;
    let throughput_hundredths = successful_count.saturating_mul(100_000) / elapsed_ms.max(1);
    println!(
        "duration_ms={elapsed_ms} successful={successful} failed={failed} throughput_hz={}.{:02} min_ms={} mean_ms={}.{} p50_ms={} p95_ms={} max_ms={}",
        throughput_hundredths / 100,
        throughput_hundredths % 100,
        round_trips[0],
        mean_tenths / 10,
        mean_tenths % 10,
        percentile(&round_trips, 50),
        percentile(&round_trips, 95),
        round_trips[successful - 1],
    );
    Ok(())
}

/* === Private Functions === */

/* === Tests === */
