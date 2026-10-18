/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Helios Mesh rotator composition root.

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
use helios_mesh_drivers::motor::{self, wing::StepperPort};
use helios_mesh_nodes::{
    composition,
    nodes::rotator::{CHANNELS, Rotator},
    satellites::MotorWing,
};
use helios_mesh_protocol::motion::{Mechanics, Microsteps, Power};

/* === Constants === */

const UNCONFIGURED: motor::Config = motor::Config {
    mechanics: Mechanics { full_steps: 200, microsteps: Microsteps::Four, reversed: false },
    power: Power { moving: 0, holding: 0 },
};
static NODE: StaticCell<node::Runtime<Rotator, board::Flash, board::Platform>> = StaticCell::new();
static WING: StaticCell<board::MotorWing> = StaticCell::new();
static COMMANDS: [motor::Commands; CHANNELS.len()] = [motor::Commands::new(), motor::Commands::new()];
static COMPLETIONS: [motor::Completions; 2] = [motor::Completions::new(), motor::Completions::new()];

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
    let hardware = composition::hardware(board::Wing::Motor(board::wings::Motor::standard()));
    let hardware = defmt::unwrap!(hardware.validate());
    let mut board = board::Board::start(spawner, Rotator::NODE_TYPE.definition().label, hardware).await;
    let node_id = board.identity.node;
    let mut boot = Boot::new();
    let radio = match board.radio.take() {
        Some(radio) => defmt::unwrap!(boot.bring_up(radio).await),
        None => None,
    };
    let display = defmt::unwrap!(composition::Display::bring_up(&mut board, &mut boot).await);
    let wing = match board.take_motor() {
        Some(motor) => defmt::unwrap!(boot.bring_up(MotorWing(motor)).await),
        None => None,
    };
    let boot = boot.seal();
    let motors = [motor::Handle::new(&COMMANDS[0]), motor::Handle::new(&COMMANDS[1])];
    let role = Rotator::initialize(spawner, &boot, motors, [&COMPLETIONS[0], &COMPLETIONS[1]]);
    let channels = role.motor_channels();
    let (node, handle) = NODE.init(node::Runtime::compose(board.identity, board.flash, board.platform, boot, role)).split();

    info!("rotator ready id={}", node_id.0);

    if let Some(wing) = wing {
        let (wing_runner, [port_a, port_b]) = WING.init(wing).into_steppers();
        let steppers = [
            motor::Stepper::new(port_a, UNCONFIGURED, COMMANDS[0].receiver(), COMPLETIONS[0].sender()),
            motor::Stepper::new(port_b, UNCONFIGURED, COMMANDS[1].receiver(), COMPLETIONS[1].sender()),
        ];
        // Motion residents run on core 1 by explicit policy: step timing
        // must stay isolated from USB, radio, and display load on core 0.
        board.secondary_core.start(move |motor_spawner| {
            let [stepper_a, stepper_b] = steppers;
            motor_spawner.spawn(defmt::unwrap!(stepper_task(stepper_a)));
            motor_spawner.spawn(defmt::unwrap!(stepper_task(stepper_b)));
            motor_spawner.spawn(defmt::unwrap!(wing_task(wing_runner)));
        });
        for channel in channels {
            channel.try_resources().expect("workers have no admitted jobs").set_diagnostics(handle.diagnostics());
        }
    }

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
async fn node_task(node: node::Runner<Rotator, board::Flash, board::Platform>) -> ! {
    node.run().await
}

#[embassy_executor::task]
async fn usb_reader_task(reader: board::UsbReader, node: node::Handle<Rotator>) -> ! {
    transport::run_usb_reader(reader, node).await
}

#[embassy_executor::task]
async fn usb_writer_task(writer: board::UsbWriter, node: node::Handle<Rotator>) -> ! {
    transport::run_usb_writer(writer, node).await
}

#[embassy_executor::task]
async fn mesh_task(link: board::RadioLink, node: node::Handle<Rotator>) -> ! {
    mesh::Responder::new(link, node).run().await
}

#[embassy_executor::task(pool_size = 2)]
async fn stepper_task(mut stepper: motor::Stepper<StepperPort>) -> ! {
    stepper.run().await
}

#[embassy_executor::task]
async fn wing_task(mut wing: board::MotorRunner) -> ! {
    wing.run().await
}

/* === Tests === */
