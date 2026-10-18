# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

# === Imports === #

# Standard Library Imports
from typing import final

# Third Party Imports
# Local Imports
from helios_mesh import NodeType, RotatorNode, SidecarNode
from helios_mesh.node import DeviceSelector, NodeHandle, NodeId, NodeInfo

# Type Checking Imports

# === Public Exports === #

__all__ = [
    "NODE_PAGE_SIZE",
    "GatewayNode",
    "NodePage",
    "NodeSummary",
]

# === Constants === #

NODE_PAGE_SIZE: int

# === Types === #

# === Classes === #

@final
class NodeSummary:
    @property
    def info(self) -> NodeInfo: ...
    @property
    def silent_for_s(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class NodePage:
    @property
    def nodes(self) -> list[NodeSummary]: ...
    @property
    def next_after(self) -> NodeId | None: ...
    def __repr__(self) -> str: ...

@final
class GatewayNode:
    @staticmethod
    async def connect(
        selector: DeviceSelector | None = None,
        *,
        port: str | None = None,
        device: NodeId | None = None,
    ) -> GatewayNode: ...
    @property
    def id(self) -> NodeId: ...
    @property
    def node(self) -> NodeHandle: ...
    async def scan_page(self, after: NodeId | None = None) -> NodePage: ...
    async def scan(self) -> list[NodeSummary]: ...
    async def scan_one(self, node_type: NodeType | None = None) -> NodeSummary: ...
    async def connect_node(self, target: NodeSummary | NodeId) -> SidecarNode | GatewayNode | RotatorNode: ...
    def remote(self, id: NodeId) -> NodeHandle: ...
    def __repr__(self) -> str: ...

# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
