/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared node actions and printers used by the command tree.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::fmt::Write;

// Third Party Imports

// Local Imports
use crate::error::Result;
use hnm::{
    NodeHandle,
    node::{AuxGpioId, AuxGpioMode, Bootloader, ChannelId, NodeInfo, SatelliteName, SatelliteReport, SatelliteState},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub(crate) fn print_info(info: &NodeInfo) {
    println!(
        "node={:06} name={:?} firmware={}.{}.{}",
        info.id.0,
        info.label(),
        info.firmware.major,
        info.firmware.minor,
        info.firmware.patch
    );
}

pub(crate) async fn describe(node: &NodeHandle) -> Result<()> {
    let description = node.describe().await?;
    print_info(&description.info);

    println!("{}", satellite_line(&description.satellites));
    Ok(())
}

pub(crate) async fn status(node: &NodeHandle) -> Result<()> {
    let status = node.status().await?;
    print_info(&status.info);
    for (index, mode) in status.auxiliary_gpio.into_iter().enumerate() {
        println!("gpio={} access={:?} mode={mode:?}", index + 1, status.auxiliary_gpio_access[index]);
    }
    Ok(())
}

pub(crate) async fn diagnostics(node: &NodeHandle) -> Result<()> {
    let diagnostics = node.diagnostics().await?;
    println!("node={:06} uptime_ms={}", diagnostics.node.0, diagnostics.uptime_ms);
    println!(
        "requests received={} succeeded={} failed={}",
        diagnostics.requests.received, diagnostics.requests.succeeded, diagnostics.requests.failed
    );
    println!(
        "journal retained={} oldest_sequence={} latest_sequence={} overwritten={}",
        diagnostics.journal.retained,
        diagnostics.journal.oldest_sequence,
        diagnostics.journal.latest_sequence,
        diagnostics.journal.overwritten
    );
    Ok(())
}

pub(crate) async fn events(node: &NodeHandle) -> Result<()> {
    let entries = node.journal().await?;
    let mut previous = None;
    for entry in entries {
        let delta = previous.map_or(0, |timestamp: u64| entry.timestamp_ms.saturating_sub(timestamp));
        println!("seq={} t={}ms +{}ms {:?}", entry.sequence, entry.timestamp_ms, delta, entry.event);
        previous = Some(entry.timestamp_ms);
    }
    Ok(())
}

pub(crate) async fn identify(node: &NodeHandle) -> Result<()> {
    node.identify().await?;
    println!("identified node {:06}", node.id().0);
    Ok(())
}

pub(crate) async fn gpio(node: &NodeHandle, gpio: u8, mode: AuxGpioMode) -> Result<()> {
    hnm::SidecarNode::from_handle(node.clone()).await?.set_gpio(AuxGpioId(gpio), mode).await?;
    println!("configured node {:06} gpio={} mode={mode:?}", node.id().0, gpio);
    Ok(())
}

pub(crate) async fn restart(node: &NodeHandle, bootloader: Option<Bootloader>) -> Result<()> {
    node.restart(bootloader).await?;
    let id = node.id();
    match bootloader {
        Some(Bootloader::Usb) => println!("USB bootloader scheduled for node {:06}", id.0),
        None => println!("restart scheduled for node {:06}", id.0),
    }
    Ok(())
}

pub(crate) async fn move_to(node: &NodeHandle, channel: u8, degrees: f64, relative: bool) -> Result<()> {
    let guard = hnm::RotatorNode::from_handle(node.clone()).await?.axes(Some(&[ChannelId(channel)])).await?;
    let axis = &guard.axes()[0];
    let snapshot = if relative { axis.turn(degrees, None, None).await } else { axis.move_to(degrees, None, None).await }?;
    println!("node={:06} channel={} operation={} state={:?}", node.id().0, channel, snapshot.id.0, snapshot.state);
    Ok(())
}

/* === Private Functions === */

fn satellite_line(satellites: &[SatelliteReport]) -> String {
    let mut line = String::from("satellites:");
    for satellite in satellites {
        write!(line, " {}={}", satellite_name(satellite.name), satellite_state(satellite.state)).expect("writing to a String cannot fail");
    }
    line
}

const fn satellite_name(name: SatelliteName) -> &'static str {
    match name {
        SatelliteName::Radio => "radio",
        SatelliteName::DisplayTft => "display-tft",
        SatelliteName::DisplayOled => "display-oled",
        SatelliteName::MotorWing => "motor-wing",
    }
}

const fn satellite_state(state: SatelliteState) -> &'static str {
    match state {
        SatelliteState::Ready => "ready",
        SatelliteState::Absent => "absent",
        SatelliteState::Faulted => "faulted",
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satellites_render_compactly() {
        let satellites = [
            SatelliteReport { name: SatelliteName::Radio, state: SatelliteState::Ready },
            SatelliteReport { name: SatelliteName::DisplayTft, state: SatelliteState::Faulted },
            SatelliteReport { name: SatelliteName::MotorWing, state: SatelliteState::Absent },
        ];

        assert_eq!(satellite_line(&satellites), "satellites: radio=ready display-tft=faulted motor-wing=absent");
    }
}
