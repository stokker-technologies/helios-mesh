# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Universal node handle, connection doors, and node wire value types."""

# === Imports === #

# Standard Library Imports
# Third Party Imports
# Local Imports
from hnm._native import AUX_GPIO_COUNT as AUX_GPIO_COUNT
from hnm._native import JOURNAL_PAGE_SIZE as JOURNAL_PAGE_SIZE
from hnm._native import MAX_CHANNELS as MAX_CHANNELS
from hnm._native import MAX_SATELLITES as MAX_SATELLITES
from hnm._native import AuxGpioAccess as AuxGpioAccess
from hnm._native import AuxGpioError as AuxGpioError
from hnm._native import AuxGpioId as AuxGpioId
from hnm._native import AuxGpioMode as AuxGpioMode
from hnm._native import Bootloader as Bootloader
from hnm._native import ChannelId as ChannelId
from hnm._native import DeviceSelector as DeviceSelector
from hnm._native import FirmwareVersion as FirmwareVersion
from hnm._native import JournalEntry as JournalEntry
from hnm._native import JournalEvent as JournalEvent
from hnm._native import JournalPage as JournalPage
from hnm._native import JournalSummary as JournalSummary
from hnm._native import NodeDescription as NodeDescription
from hnm._native import NodeDiagnostics as NodeDiagnostics
from hnm._native import NodeHandle as NodeHandle
from hnm._native import NodeId as NodeId
from hnm._native import NodeInfo as NodeInfo
from hnm._native import NodeStatus as NodeStatus
from hnm._native import Operation as Operation
from hnm._native import OperationId as OperationId
from hnm._native import OperationSnapshot as OperationSnapshot
from hnm._native import OperationState as OperationState
from hnm._native import RequestDiagnostics as RequestDiagnostics
from hnm._native import SatelliteName as SatelliteName
from hnm._native import SatelliteReport as SatelliteReport
from hnm._native import SatelliteState as SatelliteState

# Type Checking Imports

# === Public Exports === #

__all__ = [
    "AUX_GPIO_COUNT",
    "JOURNAL_PAGE_SIZE",
    "MAX_CHANNELS",
    "MAX_SATELLITES",
    "AuxGpioAccess",
    "AuxGpioError",
    "AuxGpioId",
    "AuxGpioMode",
    "Bootloader",
    "ChannelId",
    "DeviceSelector",
    "FirmwareVersion",
    "JournalEntry",
    "JournalEvent",
    "JournalPage",
    "JournalSummary",
    "NodeDescription",
    "NodeDiagnostics",
    "NodeHandle",
    "NodeId",
    "NodeInfo",
    "NodeStatus",
    "Operation",
    "OperationId",
    "OperationSnapshot",
    "OperationState",
    "RequestDiagnostics",
    "SatelliteName",
    "SatelliteReport",
    "SatelliteState",
]

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
