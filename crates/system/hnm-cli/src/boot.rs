/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Shared target resolution: attached device selection plus remote addressing.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::error::Result;
use hnm::{DeviceSelector, GatewayNode, NodeHandle, node::NodeId};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

pub(crate) struct Target {
    device: Option<NodeId>,
    port: Option<String>,
    node: Option<NodeId>,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Target {
    pub(crate) const fn new(device: Option<NodeId>, port: Option<String>, node: Option<NodeId>) -> Self {
        Self { device, port, node }
    }

    pub(crate) const fn selects_attached_device(&self) -> bool {
        self.device.is_some() || self.port.is_some()
    }

    pub(crate) const fn addresses_remote(&self) -> bool {
        self.node.is_some()
    }

    pub(crate) async fn connect(&self) -> Result<NodeHandle> {
        Ok(match self.node {
            Some(id) => GatewayNode::connect(self.selector()).await?.remote(id),
            None => NodeHandle::connect(self.selector()).await?,
        })
    }

    pub(crate) async fn connect_gateway(&self) -> Result<NodeHandle> {
        Ok(match self.node {
            Some(id) => GatewayNode::connect(self.selector()).await?.remote(id),
            None => GatewayNode::connect(self.selector()).await?.node().clone(),
        })
    }

    fn selector(&self) -> DeviceSelector {
        if let Some(port) = &self.port {
            DeviceSelector::Port(port.clone())
        } else if let Some(device) = self.device {
            DeviceSelector::Id(device)
        } else {
            DeviceSelector::Only
        }
    }
}

/* === Public Functions === */

pub(crate) fn initialize(env_file: Option<&std::path::Path>) -> Result<()> {
    let path = env_file.unwrap_or_else(|| std::path::Path::new(".env"));
    match dotenvy::from_path(path) {
        Ok(()) => Ok(()),
        Err(dotenvy::Error::Io(error)) if env_file.is_none() && error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(crate::error::Error::Environment(error)),
    }
}

/* === Private Functions === */

/* === Tests === */
