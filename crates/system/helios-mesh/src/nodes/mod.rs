/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Typed USB discovery and node handles.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::{AxesGuard, DeviceSelector, Error, GatewayNode, NodeDefinition, NodeHandle, NodeType, Result};
use helios_mesh_protocol::node::{AuxGpioId, AuxGpioMode, ChannelId, NodeId, NodeInfo};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredNode {
    pub info: NodeInfo,

    pub port: String,
}

#[derive(Clone)]
pub struct SidecarNode {
    inner: NodeHandle,
}

#[derive(Clone)]
pub struct RotatorNode {
    inner: NodeHandle,
}

/* === Enums === */

#[derive(Clone)]
pub enum ConnectedNode {
    Sidecar(SidecarNode),

    Gateway(GatewayNode),

    Rotator(RotatorNode),
}

/* === Traits === */

/* === Implementations === */

impl DiscoveredNode {
    #[must_use]
    pub const fn definition(&self) -> &'static NodeDefinition {
        self.info.node_type.definition()
    }

    pub async fn connect(&self) -> Result<ConnectedNode> {
        let node = NodeHandle::connect(DeviceSelector::Port(self.port.clone())).await?;
        let info = node.describe().await?.info;
        if info.id != self.info.id || info.node_type != self.info.node_type {
            return Err(Error::InvalidArgument("the selected USB node changed"));
        }
        Ok(ConnectedNode::from_verified(node, info.node_type))
    }
}

impl ConnectedNode {
    pub(crate) fn from_verified(node: NodeHandle, kind: NodeType) -> Self {
        match kind {
            NodeType::Sidecar => Self::Sidecar(SidecarNode { inner: node }),
            NodeType::Gateway => Self::Gateway(GatewayNode::new(node)),
            NodeType::Rotator => Self::Rotator(RotatorNode { inner: node }),
        }
    }

    #[must_use]
    pub const fn node(&self) -> &NodeHandle {
        match self {
            Self::Sidecar(node) => &node.inner,
            Self::Gateway(node) => node.node(),
            Self::Rotator(node) => &node.inner,
        }
    }

    #[must_use]
    pub const fn node_type(&self) -> NodeType {
        match self {
            Self::Sidecar(_) => NodeType::Sidecar,
            Self::Gateway(_) => NodeType::Gateway,
            Self::Rotator(_) => NodeType::Rotator,
        }
    }
}

impl SidecarNode {
    pub async fn from_handle(node: NodeHandle) -> Result<Self> {
        if node.describe().await?.info.node_type != NodeType::Sidecar {
            return Err(Error::Unsupported);
        }
        Ok(Self { inner: node })
    }

    pub async fn connect(selector: DeviceSelector) -> Result<Self> {
        Ok(Self { inner: connect_type(selector, NodeType::Sidecar).await? })
    }

    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.inner.id()
    }

    #[must_use]
    pub const fn node(&self) -> &NodeHandle {
        &self.inner
    }

    pub async fn set_gpio(&self, gpio: AuxGpioId, mode: AuxGpioMode) -> Result<()> {
        self.inner.configure_aux_gpio(gpio, mode).await
    }

    pub async fn pulse_gpio(&self, gpio: AuxGpioId, duration: std::time::Duration) -> Result<crate::Operation> {
        self.inner.pulse_aux_gpio(gpio, duration).await
    }
}

impl RotatorNode {
    pub async fn configure(&self, steppers: &[crate::StepperConfig]) -> Result<()> {
        self.inner.configure(steppers).await
    }
    pub async fn channels(&self) -> Result<Vec<crate::motion::Description>> {
        NodeHandle::channels().await
    }

    pub async fn from_handle(node: NodeHandle) -> Result<Self> {
        if node.describe().await?.info.node_type != NodeType::Rotator {
            return Err(Error::Unsupported);
        }
        Ok(Self { inner: node })
    }

    pub async fn connect(selector: DeviceSelector) -> Result<Self> {
        Ok(Self { inner: connect_type(selector, NodeType::Rotator).await? })
    }

    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.inner.id()
    }

    #[must_use]
    pub const fn node(&self) -> &NodeHandle {
        &self.inner
    }

    pub async fn axes(&self, channels: Option<&[ChannelId]>) -> Result<AxesGuard> {
        self.inner.axes(channels).await
    }
}

/* === Public Functions === */

pub async fn discover() -> Result<Vec<DiscoveredNode>> {
    let mut nodes = Vec::new();
    for device in crate::usb_devices()? {
        let node = NodeHandle::connect(DeviceSelector::Port(device.port.clone())).await?;
        let description = node.describe().await;
        node.close().await;
        let info = description?.info;
        nodes.push(DiscoveredNode { info, port: device.port });
    }
    Ok(nodes)
}

pub async fn discover_type(kind: NodeType) -> Result<Vec<DiscoveredNode>> {
    Ok(discover().await?.into_iter().filter(|node| node.info.node_type == kind).collect())
}

/* === Private Functions === */

async fn connect_type(selector: DeviceSelector, kind: NodeType) -> Result<NodeHandle> {
    let selector = if selector == DeviceSelector::Only {
        let devices = discover_type(kind).await?;
        match devices.as_slice() {
            [] => return Err(Error::DeviceNotFound),
            [device] => DeviceSelector::Port(device.port.clone()),
            _ => {
                return Err(Error::AmbiguousDevices(
                    devices
                        .iter()
                        .map(|device| crate::Candidate { id: device.info.id, label: device.definition().label.to_owned() })
                        .collect(),
                ));
            }
        }
    } else {
        selector
    };
    let node = NodeHandle::connect(selector).await?;
    if node.describe().await?.info.node_type != kind {
        return Err(Error::InvalidArgument("the selected node has a different type"));
    }
    Ok(node)
}

/* === Tests === */
