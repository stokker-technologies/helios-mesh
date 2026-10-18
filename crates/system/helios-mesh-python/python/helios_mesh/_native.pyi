# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

# === Imports === #

# Standard Library Imports
from datetime import timedelta
from types import ModuleType
from typing import ClassVar, final

# Third Party Imports
# Local Imports
from helios_mesh.gateway import NODE_PAGE_SIZE as NODE_PAGE_SIZE
from helios_mesh.gateway import GatewayNode as GatewayNode
from helios_mesh.gateway import NodePage as NodePage
from helios_mesh.gateway import NodeSummary as NodeSummary
from helios_mesh.journal import RECORD_VERSION as RECORD_VERSION
from helios_mesh.journal import SCHEMA_VERSION as SCHEMA_VERSION
from helios_mesh.journal import Cursor as Cursor
from helios_mesh.journal import Header as Header
from helios_mesh.journal import Metadata as Metadata
from helios_mesh.journal import Record as Record
from helios_mesh.journal import event_name as event_name
from helios_mesh.journal import now_us as now_us
from helios_mesh.motion import AxesGuard as AxesGuard
from helios_mesh.motion import Axis as Axis
from helios_mesh.motion import Delta as Delta
from helios_mesh.motion import Description as Description
from helios_mesh.motion import Kind as Kind
from helios_mesh.motion import StepperConfig as StepperConfig
from helios_mesh.motion import Target as Target
from helios_mesh.motion import Timing as Timing
from helios_mesh.motion.wire import Configuration as Configuration
from helios_mesh.motion.wire import Limits as Limits
from helios_mesh.motion.wire import Mechanics as Mechanics
from helios_mesh.motion.wire import Microsteps as Microsteps
from helios_mesh.motion.wire import Power as Power
from helios_mesh.motion.wire import Settings as Settings
from helios_mesh.motion.wire import StepperConfiguration as StepperConfiguration
from helios_mesh.node import AUX_GPIO_COUNT as AUX_GPIO_COUNT
from helios_mesh.node import JOURNAL_PAGE_SIZE as JOURNAL_PAGE_SIZE
from helios_mesh.node import MAX_CHANNELS as MAX_CHANNELS
from helios_mesh.node import MAX_SATELLITES as MAX_SATELLITES
from helios_mesh.node import AuxGpioAccess as AuxGpioAccess
from helios_mesh.node import AuxGpioError as AuxGpioError
from helios_mesh.node import AuxGpioId as AuxGpioId
from helios_mesh.node import AuxGpioMode as AuxGpioMode
from helios_mesh.node import Bootloader as Bootloader
from helios_mesh.node import ChannelId as ChannelId
from helios_mesh.node import DeviceSelector as DeviceSelector
from helios_mesh.node import FirmwareVersion as FirmwareVersion
from helios_mesh.node import JournalEntry as JournalEntry
from helios_mesh.node import JournalEvent as JournalEvent
from helios_mesh.node import JournalPage as JournalPage
from helios_mesh.node import JournalSummary as JournalSummary
from helios_mesh.node import NodeDescription as NodeDescription
from helios_mesh.node import NodeDiagnostics as NodeDiagnostics
from helios_mesh.node import NodeHandle as NodeHandle
from helios_mesh.node import NodeId as NodeId
from helios_mesh.node import NodeInfo as NodeInfo
from helios_mesh.node import NodeStatus as NodeStatus
from helios_mesh.node import Operation as Operation
from helios_mesh.node import OperationId as OperationId
from helios_mesh.node import OperationSnapshot as OperationSnapshot
from helios_mesh.node import OperationState as OperationState
from helios_mesh.node import RequestDiagnostics as RequestDiagnostics
from helios_mesh.node import SatelliteName as SatelliteName
from helios_mesh.node import SatelliteReport as SatelliteReport
from helios_mesh.node import SatelliteState as SatelliteState

# Type Checking Imports

# === Public Exports === #

__all__ = [
    "AUX_GPIO_COUNT",
    "JOURNAL_PAGE_SIZE",
    "MAX_CHANNELS",
    "MAX_SATELLITES",
    "NODE_PAGE_SIZE",
    "RECORD_VERSION",
    "SCHEMA_VERSION",
    "AmbiguousDeviceError",
    "AuxGpioAccess",
    "AuxGpioError",
    "AuxGpioId",
    "AuxGpioMode",
    "AuxGpioRejectedError",
    "AxesGuard",
    "Axis",
    "Bootloader",
    "BoundElsewhereError",
    "BusyError",
    "ChannelId",
    "Configuration",
    "ConflictError",
    "Cursor",
    "Delta",
    "Description",
    "DeviceNotFoundError",
    "DeviceSelector",
    "DiscoveredNode",
    "DuplicateRequestError",
    "FirmwareVersion",
    "GatewayNode",
    "Header",
    "HeliosMeshError",
    "InvalidArgumentError",
    "InvalidInputError",
    "JournalEntry",
    "JournalEvent",
    "JournalPage",
    "JournalSummary",
    "Kind",
    "Limits",
    "Mechanics",
    "Metadata",
    "Microsteps",
    "NodeDescription",
    "NodeDiagnostics",
    "NodeHandle",
    "NodeId",
    "NodeInfo",
    "NodePage",
    "NodeStatus",
    "NodeSummary",
    "NodeType",
    "NodeUnreachableError",
    "NotFoundError",
    "Operation",
    "OperationFailedError",
    "OperationId",
    "OperationSnapshot",
    "OperationState",
    "OperationStoppedError",
    "Power",
    "ProtocolMismatchError",
    "Record",
    "RequestDiagnostics",
    "RotatorNode",
    "SatelliteName",
    "SatelliteReport",
    "SatelliteState",
    "Settings",
    "SidecarNode",
    "StepperConfig",
    "StepperConfiguration",
    "Target",
    "TimeoutError",
    "Timing",
    "TransportError",
    "UnauthorizedError",
    "UnavailableError",
    "UnexpectedResponseError",
    "UnsupportedError",
    "UsbDevice",
    "_PendingProbe",
    "discover",
    "event_name",
    "gateway",
    "journal",
    "motion",
    "node",
    "now_us",
    "usb_devices",
]

# === Constants === #

gateway: ModuleType

journal: ModuleType

motion: ModuleType

node: ModuleType

# === Types === #

# === Classes === #

@final
class NodeType:
    SIDECAR: ClassVar[NodeType]
    GATEWAY: ClassVar[NodeType]
    ROTATOR: ClassVar[NodeType]
    @property
    def identifier(self) -> str: ...
    @property
    def label(self) -> str: ...
    @property
    def description(self) -> str: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __repr__(self) -> str: ...

@final
class DiscoveredNode:
    @property
    def info(self) -> NodeInfo: ...
    @property
    def node_type(self) -> NodeType: ...
    @property
    def port(self) -> str: ...
    async def connect(self) -> SidecarNode | GatewayNode | RotatorNode: ...

@final
class SidecarNode:
    @staticmethod
    async def connect(selector: DeviceSelector) -> SidecarNode: ...
    @property
    def id(self) -> NodeId: ...
    @property
    def node(self) -> NodeHandle: ...
    async def set_gpio(self, gpio: AuxGpioId, mode: AuxGpioMode) -> None: ...
    async def pulse_gpio(self, gpio: AuxGpioId, duration: timedelta) -> Operation: ...

@final
class RotatorNode:
    async def channels(self) -> list[Description]: ...
    async def configure(self, *steppers: StepperConfig) -> None: ...
    @staticmethod
    async def connect(selector: DeviceSelector) -> RotatorNode: ...
    @property
    def id(self) -> NodeId: ...
    @property
    def node(self) -> NodeHandle: ...
    async def axes(self, channels: list[ChannelId] | None = None) -> AxesGuard: ...

class HeliosMeshError(Exception): ...
class DeviceNotFoundError(HeliosMeshError): ...
class AmbiguousDeviceError(HeliosMeshError): ...
class TransportError(HeliosMeshError): ...
class TimeoutError(HeliosMeshError): ...
class ProtocolMismatchError(HeliosMeshError): ...
class UnexpectedResponseError(HeliosMeshError): ...
class InvalidArgumentError(HeliosMeshError): ...
class OperationStoppedError(HeliosMeshError): ...
class OperationFailedError(HeliosMeshError): ...
class NodeUnreachableError(HeliosMeshError): ...

class AuxGpioRejectedError(HeliosMeshError):
    gpio: AuxGpioId
    reason: AuxGpioError

class InvalidInputError(HeliosMeshError): ...
class UnsupportedError(HeliosMeshError): ...
class UnauthorizedError(HeliosMeshError): ...
class NotFoundError(HeliosMeshError): ...
class BusyError(HeliosMeshError): ...
class ConflictError(HeliosMeshError): ...
class BoundElsewhereError(HeliosMeshError): ...
class UnavailableError(HeliosMeshError): ...
class DuplicateRequestError(HeliosMeshError): ...

@final
class UsbDevice:
    @property
    def id(self) -> NodeId: ...
    @property
    def label(self) -> str: ...
    @property
    def port(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object, /) -> bool: ...

# Test-only: exercises the `awaitable()` cancellation bridge without
# hardware. Not part of the public contract; never re-exported by `helios_mesh`.
@final
class _PendingProbe:
    def __new__(cls) -> _PendingProbe: ...
    async def wait(self) -> None: ...
    @property
    def dropped(self) -> bool: ...

# === Public Functions === #

async def discover(node_type: NodeType | None = None) -> list[DiscoveredNode]: ...
def usb_devices() -> list[UsbDevice]: ...

# === Private Functions === #

# === Entry Point === #
