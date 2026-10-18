# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Universal node handle, connection doors, and node wire value types."""

# === Imports === #

# Standard Library Imports
# Third Party Imports
# Local Imports
from helios_mesh._native import AUX_GPIO_COUNT as AUX_GPIO_COUNT
from helios_mesh._native import JOURNAL_PAGE_SIZE as JOURNAL_PAGE_SIZE
from helios_mesh._native import MAX_CHANNELS as MAX_CHANNELS
from helios_mesh._native import MAX_SATELLITES as MAX_SATELLITES
from helios_mesh._native import AuxGpioAccess as AuxGpioAccess
from helios_mesh._native import AuxGpioError as AuxGpioError
from helios_mesh._native import AuxGpioId as AuxGpioId
from helios_mesh._native import AuxGpioMode as AuxGpioMode
from helios_mesh._native import Bootloader as Bootloader
from helios_mesh._native import ChannelId as ChannelId
from helios_mesh._native import DeviceSelector as DeviceSelector
from helios_mesh._native import FirmwareVersion as FirmwareVersion
from helios_mesh._native import JournalEntry as JournalEntry
from helios_mesh._native import JournalEvent as JournalEvent
from helios_mesh._native import JournalPage as JournalPage
from helios_mesh._native import JournalSummary as JournalSummary
from helios_mesh._native import NodeDescription as NodeDescription
from helios_mesh._native import NodeDiagnostics as NodeDiagnostics
from helios_mesh._native import NodeHandle as NodeHandle
from helios_mesh._native import NodeId as NodeId
from helios_mesh._native import NodeInfo as NodeInfo
from helios_mesh._native import NodeStatus as NodeStatus
from helios_mesh._native import Operation as Operation
from helios_mesh._native import OperationId as OperationId
from helios_mesh._native import OperationSnapshot as OperationSnapshot
from helios_mesh._native import OperationState as OperationState
from helios_mesh._native import RequestDiagnostics as RequestDiagnostics
from helios_mesh._native import SatelliteName as SatelliteName
from helios_mesh._native import SatelliteReport as SatelliteReport
from helios_mesh._native import SatelliteState as SatelliteState

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
