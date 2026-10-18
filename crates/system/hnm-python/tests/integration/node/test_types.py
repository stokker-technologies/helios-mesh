# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""NodeHandle identity and enum value types follow Python protocols."""

# === Imports === #

# Standard Library Imports
# Third Party Imports
import pytest

# Local Imports
from hnm.node import (
    MAX_SATELLITES,
    AuxGpioId,
    AuxGpioMode,
    Bootloader,
    ChannelId,
    NodeDescription,
    NodeId,
    OperationId,
    OperationState,
    SatelliteName,
    SatelliteReport,
    SatelliteState,
)

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def test_identity_value_protocols() -> None:
    node = NodeId(566114)
    channel = ChannelId(1)
    operation = OperationId(7)

    assert node.is_device()
    assert int(node) == 566114
    assert str(node) == "566114"
    assert node == NodeId(566114)
    assert hash(node) == hash(NodeId(566114))
    assert int(channel) == 1
    assert operation.value == 7


def test_channel_ids_based() -> None:
    with pytest.raises(ValueError, match="one-based"):
        _ = ChannelId(0)


def test_aux_gpio_bounded() -> None:
    assert AuxGpioId(1).value == 1
    assert AuxGpioId(4).value == 4
    with pytest.raises(ValueError, match="1 through 4"):
        _ = AuxGpioId(0)
    with pytest.raises(ValueError, match="1 through 4"):
        _ = AuxGpioId(5)


def test_enum_values_names() -> None:
    assert AuxGpioMode.HIGH_IMPEDANCE.name == "high_impedance"
    assert str(AuxGpioMode.DRIVE_HIGH) == "drive_high"
    assert AuxGpioMode.DRIVE_LOW == AuxGpioMode.DRIVE_LOW
    assert AuxGpioMode.DRIVE_LOW != AuxGpioMode.DRIVE_HIGH
    assert Bootloader.USB.name == "usb"
    assert SatelliteName.RADIO.name == "radio"
    assert str(SatelliteName.DISPLAY_TFT) == "display_tft"
    assert SatelliteName.DISPLAY_OLED != SatelliteName.MOTOR_WING
    assert SatelliteState.READY.name == "ready"
    assert SatelliteState.ABSENT != SatelliteState.FAULTED


def test_satellite_surface_bounded() -> None:
    assert MAX_SATELLITES == 8
    assert SatelliteReport.__module__ == "hnm.node"
    # Satellites are boot-static (sealed once, never revised) so they ride
    # NodeDescription, not the per-request dynamic NodeStatus.
    assert hasattr(NodeDescription, "satellites")


def test_operation_states_lifecycle() -> None:
    assert not OperationState.ACTIVE.is_terminal()
    assert OperationState.COMPLETED.is_terminal()
    assert OperationState.STOPPED.is_terminal()
    assert OperationState.FAILED.is_terminal()


def test_gpio_permissions() -> None:
    from hnm.node import AuxGpioAccess, AuxGpioError

    assert AuxGpioAccess.PUSH_PULL.name == "push_pull"
    assert AuxGpioAccess.INPUT_ONLY.name == "input_only"
    assert AuxGpioAccess.RESERVED != AuxGpioAccess.NOT_ALLOCATED
    assert AuxGpioError.MODE_NOT_ALLOWED.name == "mode_not_allowed"
    assert AuxGpioError.INVALID_ID != AuxGpioError.NOT_ALLOCATED
    assert AuxGpioError.RESERVED.name == "reserved"
    assert AuxGpioError.INVALID_DURATION.name == "invalid_duration"
    assert AuxGpioError.BUSY.name == "busy"


# === Private Functions === #

# === Entry Point === #
