/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `bench dispatcher`: gated two-node hardware probe for dispatcher progress

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::time::{Duration, Instant};

// Third Party Imports
use clap::Args as ClapArgs;
use tokio::task::JoinHandle;

// Local Imports
use super::percentile;
use crate::error::{Error, Result};
use helios_mesh::{
    DeviceSelector, GatewayNode, NodeHandle,
    node::{NodeId, OperationSnapshot},
    usb_devices,
};

/* === Constants === */

const DISCOVERY_ATTEMPTS: usize = 20;
const DISCOVERY_INTERVAL: Duration = Duration::from_millis(500);
const REMOTE_HEAD_START: Duration = Duration::from_millis(5);
const LOCAL_P95_BUDGET_MS: u64 = 100;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[arg(long, default_value_t = 12)]
    samples: usize,

    #[arg(long, default_value_t = 5.0)]
    motion_degrees: f64,

    #[arg(long)]
    require_hardware: bool,
}

struct MotorProbe {
    motion: JoinHandle<helios_mesh::Result<OperationSnapshot>>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(args: Args) -> Result<()> {
    let Some((gateway, direct)) = attached_pair(args.require_hardware).await? else {
        return Ok(());
    };
    let gateway_node = gateway.node().clone();
    let remote = wait_for_remote(&gateway, direct.id()).await?;

    let probe = start_motor_probe(&direct, args.motion_degrees).await?;
    let result = probe_dispatcher(&gateway_node, &remote, args.samples.max(1)).await;
    finish_motor_probe(probe).await?;
    result
}

/* === Private Functions === */

async fn attached_pair(require_hardware: bool) -> Result<Option<(GatewayNode, NodeHandle)>> {
    let gateway = match GatewayNode::connect(DeviceSelector::Only).await {
        Ok(gateway) => gateway,
        Err(error) => {
            return skip_or_fail(require_hardware, format!("dispatcher hardware bench requires a directly attached coordinator: {error}"))
                .map(|()| None);
        }
    };
    let mut motions = Vec::new();
    for device in usb_devices()? {
        if device.id == gateway.id() {
            continue;
        }
        let Ok(node) = NodeHandle::connect(DeviceSelector::Id(device.id)).await else {
            continue;
        };
        let Ok(description) = node.describe().await else {
            continue;
        };
        if description.info.node_type == helios_mesh::NodeType::Rotator {
            motions.push(node);
        }
    }
    if motions.len() == 1 {
        return Ok(Some((gateway, motions.remove(0))));
    }
    skip_or_fail(
        require_hardware,
        format!(
            "dispatcher hardware bench requires exactly one coordinator and one distinct motion \
             node; found 1 coordinator and {} motion node(s)",
            motions.len()
        ),
    )
    .map(|()| None)
}

fn skip_or_fail(require_hardware: bool, message: String) -> Result<()> {
    if require_hardware {
        return Err(Error::Bench(message));
    }
    println!("skipped=true reason={message:?}");
    Ok(())
}

async fn wait_for_remote(gateway: &GatewayNode, id: NodeId) -> Result<NodeHandle> {
    for _ in 0..DISCOVERY_ATTEMPTS {
        if let Ok(nodes) = gateway.scan().await
            && nodes.iter().any(|summary| summary.info.id == id)
        {
            return Ok(gateway.remote(id));
        }
        tokio::time::sleep(DISCOVERY_INTERVAL).await;
    }
    Err(helios_mesh::Error::NotFound.into())
}

async fn start_motor_probe(node: &NodeHandle, degrees: f64) -> Result<Option<MotorProbe>> {
    match prepare_motor(node, degrees).await {
        Ok(probe) => Ok(Some(probe)),
        Err(Error::HeliosMesh(helios_mesh::Error::Conflict)) => {
            println!("motor_core=skipped_unconfigured");
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

async fn prepare_motor(node: &NodeHandle, degrees: f64) -> Result<MotorProbe> {
    let channels = helios_mesh::RotatorNode::from_handle(node.clone()).await?.channels().await?;
    let first = channels.first().ok_or(helios_mesh::Error::NotFound)?.channel;
    let axes = helios_mesh::RotatorNode::from_handle(node.clone()).await?.axes(Some(&[first])).await?;
    axes.axes()[0].zero().await?;
    let operation = axes.axes()[0].start_turn(degrees, None, None).await?;
    let motion = tokio::spawn(async move { operation.completed().await });
    Ok(MotorProbe { motion })
}

async fn finish_motor_probe(probe: Option<MotorProbe>) -> Result<()> {
    let Some(probe) = probe else {
        return Ok(());
    };
    let status = probe.motion.await.map_err(|error| Error::Bench(error.to_string()))??;
    println!("motor_core=exercised operation={} state={:?}", status.id.0, status.state);
    Ok(())
}

async fn probe_dispatcher(gateway: &NodeHandle, remote: &NodeHandle, samples: usize) -> Result<()> {
    let mut local_ms = Vec::with_capacity(samples);
    let mut remote_ms = Vec::with_capacity(samples);
    let mut local_first = 0;
    for _ in 0..samples {
        let epoch = Instant::now();
        let remote = remote.clone();
        let remote_request = tokio::spawn(async move {
            let response = remote.status().await;
            (epoch.elapsed(), response)
        });
        tokio::time::sleep(REMOTE_HEAD_START).await;
        let local_started = Instant::now();
        gateway.status().await?;
        let local_latency = milliseconds(local_started.elapsed());
        let local_completed = epoch.elapsed();
        let (remote_completed, response) = remote_request.await.map_err(|error| Error::Bench(error.to_string()))?;
        response?;
        if local_completed < remote_completed {
            local_first += 1;
        }
        local_ms.push(local_latency);
        remote_ms.push(milliseconds(remote_completed));
    }

    local_ms.sort_unstable();
    remote_ms.sort_unstable();
    let local_p95 = percentile(&local_ms, 95);
    println!(
        "samples={samples} local_first={local_first} local_p50_ms={} local_p95_ms={local_p95} local_max_ms={} remote_p50_ms={} remote_p95_ms={} remote_max_ms={}",
        percentile(&local_ms, 50),
        local_ms[samples - 1],
        percentile(&remote_ms, 50),
        percentile(&remote_ms, 95),
        remote_ms[samples - 1],
    );
    if local_first != samples {
        return Err(Error::Bench("local requests did not complete before their paired remote requests".into()));
    }
    if local_p95 > LOCAL_P95_BUDGET_MS {
        return Err(Error::Bench(format!("local p95 {local_p95}ms exceeds the {LOCAL_P95_BUDGET_MS}ms budget")));
    }
    Ok(())
}

fn milliseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/* === Tests === */
