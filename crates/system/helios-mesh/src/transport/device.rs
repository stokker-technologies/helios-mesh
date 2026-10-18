/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Enumeration of directly attached Helios Mesh USB devices.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use tokio_serial::{SerialPortInfo, SerialPortType};

// Local Imports
use crate::{Error, Result};
use helios_mesh_protocol::{
    node::NodeId,
    usb::{PRODUCT_ID, PRODUCT_PREFIX, VENDOR_ID},
};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsbDevice {
    pub id: NodeId,

    pub label: String,

    pub port: String,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

/* === Public Functions === */

pub fn usb_devices() -> Result<Vec<UsbDevice>> {
    let ports = tokio_serial::available_ports().map_err(|error| Error::Device(error.to_string()))?;
    Ok(ports.iter().filter_map(device_from_port).collect())
}

/* === Private Functions === */

fn device_from_port(port: &SerialPortInfo) -> Option<UsbDevice> {
    let SerialPortType::UsbPort(usb) = &port.port_type else {
        return None;
    };
    if usb.vid != VENDOR_ID || usb.pid != PRODUCT_ID {
        return None;
    }
    let serial = usb.serial_number.as_deref()?;
    if serial.len() != 6 || !serial.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = NodeId(serial.parse().ok()?);
    if !id.is_device() {
        return None;
    }
    let label = usb
        .product
        .as_deref()
        .and_then(|product| product.strip_prefix(PRODUCT_PREFIX))
        .map(|label| label.trim_start().to_owned())
        .unwrap_or_default();
    Some(UsbDevice { id, label, port: port.port_name.clone() })
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use tokio_serial::UsbPortInfo;

    use super::*;

    #[test]
    fn recognizes_usb_names() {
        let gateway = usb_port("/dev/ttyACM0", "273282", Some("Helios Node Mesh Gateway"));
        let unnamed = usb_port("/dev/ttyACM1", "641905", None);

        let gateway = device_from_port(&gateway).unwrap();
        assert_eq!(gateway.id, NodeId(273_282));
        assert_eq!(gateway.label, "Gateway");
        let unnamed = device_from_port(&unnamed).unwrap();
        assert_eq!(unnamed.id, NodeId(641_905));
        assert_eq!(unnamed.label, "");
    }

    #[test]
    fn ignores_other_devices() {
        let other = SerialPortInfo {
            port_name: "/dev/ttyUSB0".into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 1,
                pid: 2,
                serial_number: Some("273282".into()),
                manufacturer: None,
                product: Some("Helios Node Mesh Gateway".into()),
            }),
        };
        let malformed = usb_port("/dev/ttyACM0", "27328x", Some("Helios Node Mesh Gateway"));

        assert!(device_from_port(&other).is_none());
        assert!(device_from_port(&malformed).is_none());
    }

    fn usb_port(path: &str, serial: &str, product: Option<&str>) -> SerialPortInfo {
        SerialPortInfo {
            port_name: path.into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: VENDOR_ID,
                pid: PRODUCT_ID,
                serial_number: Some(serial.into()),
                manufacturer: None,
                product: product.map(Into::into),
            }),
        }
    }
}
