/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Helios Node Mesh gateway composition root.

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
use hnm_board_feather_rp2040 as board;
use hnm_core::{
    boot::Boot,
    mesh,
    node::{self, Node as _},
    transport,
};
use hnm_nodes::{composition, nodes::gateway::Gateway};

/* === Constants === */

static NODE: StaticCell<node::Runtime<Gateway, board::Flash, board::Platform>> = StaticCell::new();
static COORDINATOR: StaticCell<Mesh> = StaticCell::new();

/* === Macros === */

/* === Type Aliases === */

type Mesh = mesh::Coordinator<board::RadioKind, board::RadioDelay>;
type MeshRunner = mesh::coordinator::Runner<board::RadioKind, board::RadioDelay>;

/* === Structs === */

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

/* === Private Functions === */

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let hardware = composition::hardware(board::Wing::Tft(board::wings::TftV2::standard()));
    let hardware = defmt::unwrap!(hardware.validate());
    let mut board = board::Board::start(spawner, Gateway::NODE_TYPE.definition().label, hardware).await;
    let node_id = board.identity.node;
    let mut boot = Boot::new();
    let radio = match board.radio.take() {
        Some(radio) => defmt::unwrap!(boot.bring_up(radio).await),
        None => None,
    };
    let display = defmt::unwrap!(composition::Display::bring_up(&mut board, &mut boot).await);
    let boot = boot.seal();
    let role = Gateway;
    let mesh = radio.map(|link| COORDINATOR.init(mesh::Coordinator::new(link)).split());
    let node = node::Runtime::compose(board.identity, board.flash, board.platform, boot, role);
    let node = match &mesh {
        Some((_, forward)) => node.forward_to(*forward),
        None => node,
    };
    let (node, handle) = NODE.init(node).split();

    info!("gateway ready id={}", node_id.0);

    let (usb_reader, usb_writer) = board.usb;
    spawner.spawn(defmt::unwrap!(usb_reader_task(usb_reader, handle)));
    spawner.spawn(defmt::unwrap!(usb_writer_task(usb_writer, handle)));
    if let Some((mesh, _)) = mesh {
        spawner.spawn(defmt::unwrap!(coordinator_task(mesh, handle)));
    }
    if let (Some(display), Some(snapshots)) = (display, handle.ui()) {
        display.spawn(spawner, snapshots);
    }
    spawner.spawn(defmt::unwrap!(node_task(node)));
    core::future::pending::<()>().await;
}

#[embassy_executor::task]
async fn node_task(node: node::Runner<Gateway, board::Flash, board::Platform>) -> ! {
    node.run().await
}

#[embassy_executor::task]
async fn usb_reader_task(reader: board::UsbReader, node: node::Handle<Gateway>) -> ! {
    transport::run_usb_reader(reader, node).await
}

#[embassy_executor::task]
async fn usb_writer_task(writer: board::UsbWriter, node: node::Handle<Gateway>) -> ! {
    transport::run_usb_writer(writer, node).await
}

#[embassy_executor::task]
async fn coordinator_task(mesh: MeshRunner, node: node::Handle<Gateway>) -> ! {
    mesh.run(node).await
}

/* === Tests === */
