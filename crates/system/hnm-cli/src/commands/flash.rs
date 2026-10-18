/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// `flash`: write a UF2 image onto an RP2040 in BOOTSEL mode.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

// Third Party Imports
use clap::Args as ClapArgs;
use tokio::time::{Instant, sleep};

// Local Imports
use crate::{
    boot::Target,
    bootsel,
    error::{Error, Result},
    node_action,
};
use hnm::{
    DeviceSelector, NodeHandle,
    node::{Bootloader, NodeId},
};

/* === Constants === */

const VOLUME_TIMEOUT: Duration = Duration::from_secs(30);
const REENUMERATION_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const HEARTBEAT_TICKS: u32 = 10;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    uf2: PathBuf,
}

/* === Enums === */

enum RestartOutcome {
    Confirmed,

    Degraded { host: u8, device: Option<u8> },
}

#[derive(Clone, Copy)]
enum Progress {
    Silent,

    Announce,
}

enum VolumePoll {
    Ready(Result<PathBuf>),

    NotYet(Option<Error>),
}

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) async fn run(target: &Target, args: Args) -> Result<()> {
    if target.addresses_remote() {
        return Err(Error::Flash("flash writes over USB; --node is not supported".into()));
    }
    if !args.uf2.is_file() {
        return Err(Error::Flash(format!("UF2 image {} not found", args.uf2.display())));
    }
    if target.selects_attached_device() { trigger_and_flash(target, &args.uf2).await } else { flash_via_volume(&args.uf2) }
}

/* === Private Functions === */

async fn trigger_and_flash(target: &Target, uf2: &Path) -> Result<()> {
    let mounted = bootsel::discover()?;
    if let Some(volume) = mounted.first() {
        return Err(Error::Flash(format!(
            "a BOOTSEL volume is already mounted at {}; flash it without \
             --device/--port, or eject it first",
            volume.display()
        )));
    }
    let node = target.connect().await?;
    let id = node.id();
    let restart = node_action::restart(&node, Some(Bootloader::Usb)).await;
    let progress = match classify_restart(restart)? {
        RestartOutcome::Confirmed => Progress::Silent,
        RestartOutcome::Degraded { host, device } => {
            warn_protocol_mismatch(host, device);
            Progress::Announce
        }
    };
    let volume = wait_for_volume(progress).await?;
    flash_volume(uf2, &volume)?;
    let node = wait_for_node(id).await?;
    node_action::status(&node).await
}

fn classify_restart(result: Result<()>) -> Result<RestartOutcome> {
    match result {
        Ok(()) => Ok(RestartOutcome::Confirmed),
        Err(Error::Hnm(hnm::Error::ProtocolMismatch { host, device })) => Ok(RestartOutcome::Degraded { host, device }),
        Err(error) => Err(error),
    }
}

fn warn_protocol_mismatch(host: u8, device: Option<u8>) {
    match device {
        Some(version) => eprintln!(
            "protocol version mismatch on the restart request (host speaks {host}, \
             device answered at version {version}); the device rejected the request \
             before acting on it, so it did not restart"
        ),
        None => eprintln!(
            "protocol version mismatch on the restart request (host speaks {host}); \
             the device rejected the request before acting on it, so it did not restart"
        ),
    }
    eprintln!(
        "press and hold BOOTSEL on the board (or tap RESET while holding BOOTSEL) to \
         enter the bootloader — waiting for it to appear"
    );
}

fn flash_via_volume(uf2: &Path) -> Result<()> {
    bootsel::discover().and_then(bootsel::exactly_one).and_then(|volume| flash_volume(uf2, &volume))
}

fn flash_volume(uf2: &Path, volume: &Path) -> Result<()> {
    let destination = bootsel::write_uf2(uf2, volume)?;
    println!("flashed {}", destination.display());
    Ok(())
}

fn classify_volume_poll(outcome: Result<Vec<PathBuf>>) -> VolumePoll {
    match outcome {
        Ok(volumes) if volumes.is_empty() => VolumePoll::NotYet(None),
        Ok(volumes) => VolumePoll::Ready(bootsel::exactly_one(volumes)),
        Err(error) => VolumePoll::NotYet(Some(error)),
    }
}

async fn wait_for_volume(progress: Progress) -> Result<PathBuf> {
    let deadline = Instant::now() + VOLUME_TIMEOUT;
    // Sticky: once discovery reports a diagnostic (for example `udisksctl`
    // racing the automounter), keep it through later empty-but-error-free
    // polls, so a timeout after the race clears still explains what raced.
    let mut last_error: Option<Error> = None;
    let mut tick = 0_u32;
    loop {
        last_error = match classify_volume_poll(bootsel::discover()) {
            VolumePoll::Ready(result) => return result,
            VolumePoll::NotYet(error) => error.or(last_error),
        };
        if Instant::now() >= deadline {
            return Err(last_error.unwrap_or_else(|| {
                Error::Flash(format!("no RPI-RP2 BOOTSEL volume appeared within {} seconds", VOLUME_TIMEOUT.as_secs()))
            }));
        }
        maybe_heartbeat(progress, tick, "a BOOTSEL volume");
        tick += 1;
        sleep(POLL_INTERVAL).await;
    }
}

fn maybe_heartbeat(progress: Progress, tick: u32, subject: &str) {
    if matches!(progress, Progress::Announce) && tick > 0 && tick.is_multiple_of(HEARTBEAT_TICKS) {
        let elapsed = (POLL_INTERVAL * tick).as_secs();
        eprintln!(
            "... still waiting for {subject} ({elapsed}s elapsed) — press BOOTSEL on \
             the board if it has not restarted"
        );
    }
}

async fn wait_for_node(id: NodeId) -> Result<NodeHandle> {
    let deadline = Instant::now() + REENUMERATION_TIMEOUT;
    loop {
        if let Ok(node) = NodeHandle::connect(DeviceSelector::Id(id)).await
            && node.status().await.is_ok()
        {
            return Ok(node);
        }
        if Instant::now() >= deadline {
            return Err(Error::Flash(format!(
                "node {:06} did not answer within {} seconds of flashing",
                id.0,
                REENUMERATION_TIMEOUT.as_secs()
            )));
        }
        sleep(POLL_INTERVAL).await;
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_confirmation_silently() {
        assert!(matches!(classify_restart(Ok(())), Ok(RestartOutcome::Confirmed)));
    }

    #[test]
    fn protocol_mismatch_it() {
        let result = classify_restart(Err(Error::Hnm(hnm::Error::ProtocolMismatch { host: 3, device: Some(2) })));
        assert!(matches!(result, Ok(RestartOutcome::Degraded { host: 3, device: Some(2) })));
    }

    #[test]
    fn other_restart_fatal() {
        let result = classify_restart(Err(Error::Hnm(hnm::Error::Timeout)));
        assert!(matches!(result, Err(Error::Hnm(hnm::Error::Timeout))));
    }

    #[test]
    fn volume_poll_yet() {
        assert!(matches!(classify_volume_poll(Ok(Vec::new())), VolumePoll::NotYet(None)));
    }

    #[test]
    fn volume_poll_diagnostic() {
        let error = Error::Flash("udisksctl could not mount BOOTSEL device /dev/sda1: Device or resource busy".into());
        match classify_volume_poll(Err(error)) {
            VolumePoll::NotYet(Some(kept)) => {
                assert!(kept.to_string().contains("udisksctl"));
            }
            _ => panic!("expected NotYet to retain the discovery diagnostic"),
        }
    }

    #[test]
    fn volume_poll_found() {
        let volume = PathBuf::from("/run/media/user/RPI-RP2");
        match classify_volume_poll(Ok(vec![volume.clone()])) {
            VolumePoll::Ready(Ok(found)) => assert_eq!(found, volume),
            _ => panic!("expected a ready volume"),
        }
    }

    #[test]
    fn volume_poll_mounted() {
        let volumes = vec![PathBuf::from("/run/media/user/RPI-RP2"), PathBuf::from("/run/media/user/RPI-RP21")];
        match classify_volume_poll(Ok(volumes)) {
            VolumePoll::Ready(Err(error)) => assert!(error.to_string().contains("found 2")),
            _ => panic!("expected a ready ambiguity error, not another retry"),
        }
    }
}
