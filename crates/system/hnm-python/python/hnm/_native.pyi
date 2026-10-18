# ***
#
# Helios Node Mesh
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
from hnm.gateway import NODE_PAGE_SIZE as NODE_PAGE_SIZE
from hnm.gateway import GatewayNode as GatewayNode
from hnm.gateway import NodePage as NodePage
from hnm.gateway import NodeSummary as NodeSummary
from hnm.journal import RECORD_VERSION as RECORD_VERSION
from hnm.journal import SCHEMA_VERSION as SCHEMA_VERSION
from hnm.journal import Cursor as Cursor
from hnm.journal import Header as Header
from hnm.journal import Metadata as Metadata
from hnm.journal import Record as Record
from hnm.journal import event_name as event_name
from hnm.journal import now_us as now_us
from hnm.motion import AxesGuard as AxesGuard
from hnm.motion import Axis as Axis
from hnm.motion import Delta as Delta
from hnm.motion import Description as Description
from hnm.motion import Kind as Kind
from hnm.motion import StepperConfig as StepperConfig
from hnm.motion import Target as Target
from hnm.motion import Timing as Timing
from hnm.motion.wire import Configuration as Configuration
from hnm.motion.wire import Limits as Limits
from hnm.motion.wire import Mechanics as Mechanics
from hnm.motion.wire import Microsteps as Microsteps
from hnm.motion.wire import Power as Power
from hnm.motion.wire import Settings as Settings
from hnm.motion.wire import StepperConfiguration as StepperConfiguration
from hnm.node import AUX_GPIO_COUNT as AUX_GPIO_COUNT
from hnm.node import JOURNAL_PAGE_SIZE as JOURNAL_PAGE_SIZE
from hnm.node import MAX_CHANNELS as MAX_CHANNELS
from hnm.node import MAX_SATELLITES as MAX_SATELLITES
from hnm.node import AuxGpioAccess as AuxGpioAccess
from hnm.node import AuxGpioError as AuxGpioError
from hnm.node import AuxGpioId as AuxGpioId
from hnm.node import AuxGpioMode as AuxGpioMode
from hnm.node import Bootloader as Bootloader
from hnm.node import ChannelId as ChannelId
from hnm.node import DeviceSelector as DeviceSelector
from hnm.node import FirmwareVersion as FirmwareVersion
from hnm.node import JournalEntry as JournalEntry
from hnm.node import JournalEvent as JournalEvent
from hnm.node import JournalPage as JournalPage
from hnm.node import JournalSummary as JournalSummary
from hnm.node import NodeDescription as NodeDescription
from hnm.node import NodeDiagnostics as NodeDiagnostics
from hnm.node import NodeHandle as NodeHandle
from hnm.node import NodeId as NodeId
from hnm.node import NodeInfo as NodeInfo
from hnm.node import NodeStatus as NodeStatus
from hnm.node import Operation as Operation
from hnm.node import OperationId as OperationId
from hnm.node import OperationSnapshot as OperationSnapshot
from hnm.node import OperationState as OperationState
from hnm.node import RequestDiagnostics as RequestDiagnostics
from hnm.node import SatelliteName as SatelliteName
from hnm.node import SatelliteReport as SatelliteReport
from hnm.node import SatelliteState as SatelliteState

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
    "HnmError",
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

class HnmError(Exception): ...
class DeviceNotFoundError(HnmError): ...
class AmbiguousDeviceError(HnmError): ...
class TransportError(HnmError): ...
class TimeoutError(HnmError): ...
class ProtocolMismatchError(HnmError): ...
class UnexpectedResponseError(HnmError): ...
class InvalidArgumentError(HnmError): ...
class OperationStoppedError(HnmError): ...
class OperationFailedError(HnmError): ...
class NodeUnreachableError(HnmError): ...

class AuxGpioRejectedError(HnmError):
    gpio: AuxGpioId
    reason: AuxGpioError

class InvalidInputError(HnmError): ...
class UnsupportedError(HnmError): ...
class UnauthorizedError(HnmError): ...
class NotFoundError(HnmError): ...
class BusyError(HnmError): ...
class ConflictError(HnmError): ...
class BoundElsewhereError(HnmError): ...
class UnavailableError(HnmError): ...
class DuplicateRequestError(HnmError): ...

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
# hardware. Not part of the public contract; never re-exported by `hnm`.
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
