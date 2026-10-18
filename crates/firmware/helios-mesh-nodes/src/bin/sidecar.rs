/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Helios Mesh sidecar composition root.

#![no_std]
#![no_main]

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use panic_probe as _;
use static_cell::StaticCell;

// Local Imports
use helios_mesh_board_feather_rp2040 as board;
use helios_mesh_core::{
    boot::Boot,
    mesh,
    node::{self, Node as _},
    transport,
};
use helios_mesh_nodes::{composition, nodes::sidecar::Sidecar};
use helios_mesh_protocol::node::{AuxGpioId, AuxGpioMode};

/* === Constants === */

static NODE: StaticCell<node::Runtime<Sidecar, board::Flash, board::Platform>> = StaticCell::new();

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let hardware = composition::hardware(board::Wing::Oled(board::wings::Oled::standard()));
    let hardware = if cfg!(feature = "hardware-unknown") {
        hardware
    } else {
        hardware
            .auxiliary(AuxGpioId(1), board::Auxiliary::PushPull { initial: AuxGpioMode::HighImpedance, drive: board::Drive::Ma2 })
            .auxiliary(AuxGpioId(2), board::Auxiliary::PushPull { initial: AuxGpioMode::HighImpedance, drive: board::Drive::Ma2 })
    };
    let hardware = defmt::unwrap!(hardware.validate());
    let mut board = board::Board::start(spawner, Sidecar::NODE_TYPE.definition().label, hardware).await;
    let node_id = board.identity.node;
    let mut boot = Boot::new();
    let radio = match board.radio.take() {
        Some(radio) => defmt::unwrap!(boot.bring_up(radio).await),
        None => None,
    };
    let display = defmt::unwrap!(composition::Display::bring_up(&mut board, &mut boot).await);
    let boot = boot.seal();
    let sidecar = Sidecar::initialize(&mut board, spawner);
    let (node, handle) = NODE.init(node::Runtime::compose(board.identity, board.flash, board.platform, boot, sidecar)).split();

    info!("sidecar ready id={}", node_id.0);

    let (usb_reader, usb_writer) = board.usb;
    spawner.spawn(defmt::unwrap!(usb_reader_task(usb_reader, handle)));
    spawner.spawn(defmt::unwrap!(usb_writer_task(usb_writer, handle)));
    if let Some(link) = radio {
        spawner.spawn(defmt::unwrap!(mesh_task(link, handle)));
    }
    if let (Some(display), Some(snapshots)) = (display, handle.ui()) {
        display.spawn(spawner, snapshots);
    }
    spawner.spawn(defmt::unwrap!(node_task(node)));
    core::future::pending::<()>().await;
}

#[embassy_executor::task]
async fn node_task(node: node::Runner<Sidecar, board::Flash, board::Platform>) -> ! {
    node.run().await
}

#[embassy_executor::task]
async fn usb_reader_task(reader: board::UsbReader, node: node::Handle<Sidecar>) -> ! {
    transport::run_usb_reader(reader, node).await
}

#[embassy_executor::task]
async fn usb_writer_task(writer: board::UsbWriter, node: node::Handle<Sidecar>) -> ! {
    transport::run_usb_writer(writer, node).await
}

#[embassy_executor::task]
async fn mesh_task(link: board::RadioLink, node: node::Handle<Sidecar>) -> ! {
    mesh::Responder::new(link, node).run().await
}

/* === Tests === */
