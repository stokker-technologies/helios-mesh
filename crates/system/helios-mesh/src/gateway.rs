/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Mesh-brokerage capability view: coordinator connection, roster scanning,

/* === Modules === */

/* === Re-Exports === */

pub use helios_mesh_protocol::gateway::{NODE_PAGE_SIZE, NodePage};
pub use helios_mesh_protocol::node::NodeSummary;

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::{
    Error, Result,
    node::{DeviceSelector, NodeHandle, NodeId},
};
use helios_mesh_protocol::gateway;

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone)]
pub struct GatewayNode {
    node: NodeHandle,
}

/* === Enums === */

pub enum NodeTarget {
    Summary(NodeSummary),

    Id(NodeId),
}

/* === Traits === */

/* === Implementations === */

impl From<NodeSummary> for NodeTarget {
    fn from(summary: NodeSummary) -> Self {
        Self::Summary(summary)
    }
}

impl From<NodeId> for NodeTarget {
    fn from(id: NodeId) -> Self {
        Self::Id(id)
    }
}

impl GatewayNode {
    pub async fn from_handle(node: NodeHandle) -> Result<Self> {
        node.gateway().await
    }

    pub(crate) const fn new(node: NodeHandle) -> Self {
        Self { node }
    }

    pub async fn connect(selector: DeviceSelector) -> Result<Self> {
        NodeHandle::connect_gateway(selector).await?.gateway().await
    }

    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.node.id()
    }

    #[must_use]
    pub const fn node(&self) -> &NodeHandle {
        &self.node
    }

    pub async fn scan_page(&self, after: Option<NodeId>) -> Result<NodePage> {
        self.node.request(gateway::Nodes { after }).await
    }

    pub async fn scan(&self) -> Result<Vec<NodeSummary>> {
        let mut nodes = Vec::new();
        let mut after = None;
        loop {
            let page = self.scan_page(after).await?;
            nodes.extend(page.nodes);
            let Some(next_after) = page.next_after else {
                return Ok(nodes);
            };
            if after.is_some_and(|after: NodeId| next_after.0 <= after.0) {
                return Err(Error::UnexpectedResponse);
            }
            after = Some(next_after);
        }
    }

    pub async fn scan_one(&self, node_type: Option<crate::NodeType>) -> Result<NodeSummary> {
        let matches: Vec<NodeSummary> =
            self.scan().await?.into_iter().filter(|summary| node_type.is_none_or(|kind| summary.info.node_type == kind)).collect();
        match <[NodeSummary; 1]>::try_from(matches) {
            Ok([summary]) => Ok(summary),
            Err(matches) if matches.is_empty() => Err(Error::NotFound),
            Err(matches) => Err(Error::AmbiguousDevices(matches.iter().map(Into::into).collect())),
        }
    }

    pub async fn connect_node(&self, target: impl Into<NodeTarget>) -> Result<crate::ConnectedNode> {
        let id = match target.into() {
            NodeTarget::Summary(summary) => {
                return Ok(crate::ConnectedNode::from_verified(self.remote(summary.info.id), summary.info.node_type));
            }
            NodeTarget::Id(id) => id,
        };
        let node = self.remote(id);
        let info = node.describe().await?.info;
        if info.id != id {
            return Err(Error::UnexpectedResponse);
        }
        Ok(crate::ConnectedNode::from_verified(node, info.node_type))
    }

    #[must_use]
    pub fn remote(&self, id: NodeId) -> NodeHandle {
        self.node.peer(id)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */
