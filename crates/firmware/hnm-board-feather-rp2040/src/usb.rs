/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Feather RP2040 USB CDC device construction and raw packet halves.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use embassy_rp::{peripherals::USB, usb::Driver};
use embassy_usb::{
    Builder, Config, UsbDevice,
    class::cdc_acm::{CdcAcmClass, Receiver, Sender, State},
    driver::EndpointError,
};
use static_cell::StaticCell;

// Local Imports
use hnm_core::{
    identity,
    transport::{FrameRead, FrameWrite},
};
use hnm_protocol::{
    node::NodeId,
    usb::{MANUFACTURER, MAX_PRODUCT, PRODUCT_ID, VENDOR_ID, product},
};

/* === Constants === */

const PACKET_SIZE: usize = 64;
const CDC_PACKET_SIZE: u16 = 64;
const CONTROL_PACKET_SIZE: u8 = 64;
static PRODUCT: StaticCell<heapless::String<MAX_PRODUCT>> = StaticCell::new();
static SERIAL: StaticCell<[u8; 6]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<State<'static>> = StaticCell::new();

/* === Macros === */

/* === Type Aliases === */

type UsbDriver = Driver<'static, USB>;

/* === Structs === */

pub struct UsbReader {
    receiver: Receiver<'static, UsbDriver>,
}

pub struct UsbWriter {
    sender: Sender<'static, UsbDriver>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl FrameRead for UsbReader {
    type Error = EndpointError;

    async fn wait_connected(&mut self) {
        self.receiver.wait_connection().await;
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.receiver.read_packet(buffer).await
    }
}

impl FrameWrite for UsbWriter {
    type Error = EndpointError;

    async fn wait_connected(&mut self) {
        self.sender.wait_connection().await;
    }

    async fn write(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
        for packet in frame.chunks(PACKET_SIZE) {
            self.sender.write_packet(packet).await?;
        }
        Ok(())
    }
}

/* === Public Functions === */

pub(crate) fn build(driver: UsbDriver, node_id: NodeId, product_name: &str) -> (UsbDevice<'static, UsbDriver>, (UsbReader, UsbWriter)) {
    let mut config = Config::new(VENDOR_ID, PRODUCT_ID);
    config.manufacturer = Some(MANUFACTURER);
    config.product = Some(PRODUCT.init(product(product_name)).as_str());
    let serial = core::str::from_utf8(SERIAL.init(identity::decimal_digits(node_id))).expect("device ID contains only ASCII digits");
    config.serial_number = Some(serial);
    config.max_power = 100;
    config.max_packet_size_0 = CONTROL_PACKET_SIZE;

    let mut builder =
        Builder::new(driver, config, CONFIG_DESCRIPTOR.init([0; 256]), BOS_DESCRIPTOR.init([0; 256]), &mut [], CONTROL.init([0; 64]));
    let class = CdcAcmClass::new(&mut builder, CDC_STATE.init(State::new()), CDC_PACKET_SIZE);
    let device = builder.build();
    let (sender, receiver) = class.split();
    (device, (UsbReader { receiver }, UsbWriter { sender }))
}

#[embassy_executor::task]
pub(crate) async fn device_task(mut device: UsbDevice<'static, UsbDriver>) -> ! {
    device.run().await
}

/* === Private Functions === */

/* === Tests === */
