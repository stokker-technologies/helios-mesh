# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""The two connection doors and typed exception mapping."""

# === Imports === #

# Standard Library Imports
from asyncio import run

# Third Party Imports
import pytest

# Local Imports
import hnm
from hnm import DeviceNotFoundError, GatewayNode, HnmError, NodeHandle
from hnm.node import DeviceSelector, NodeId

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def test_connecting_missing_error() -> None:
    async def connect_missing() -> None:
        _ = await NodeHandle.connect(DeviceSelector.id(NodeId(998877)))

    with pytest.raises(DeviceNotFoundError, match="was not found"):
        run(connect_missing())


def test_connecting_missing_gateway() -> None:
    async def connect_missing() -> None:
        _ = await GatewayNode.connect(DeviceSelector.id(NodeId(998877)))

    with pytest.raises(DeviceNotFoundError, match="was not found"):
        run(connect_missing())


def test_port_device_positional() -> None:
    async def connect_by_port() -> None:
        _ = await NodeHandle.connect(port="/dev/does-not-exist")

    async def connect_by_device() -> None:
        _ = await NodeHandle.connect(device=NodeId(998877))

    with pytest.raises(DeviceNotFoundError):
        run(connect_by_port())
    with pytest.raises(DeviceNotFoundError, match="was not found"):
        run(connect_by_device())


def test_selector_kwargs_exclusive() -> None:
    async def connect_with_both() -> None:
        _ = await NodeHandle.connect(DeviceSelector.only(), port="/dev/ttyACM0")

    with pytest.raises(ValueError, match="at most one"):
        run(connect_with_both())


def test_device_selectors_semantics() -> None:
    assert DeviceSelector.only() == DeviceSelector.only()
    assert DeviceSelector.id(NodeId(273282)) == DeviceSelector.id(NodeId(273282))
    assert DeviceSelector.port("/dev/ttyACM0") != DeviceSelector.only()
    assert repr(DeviceSelector.port("/dev/ttyACM0")) == 'DeviceSelector.port("/dev/ttyACM0")'


def test_exception_category_base() -> None:
    exceptions = [name for name in hnm.__all__ if name.endswith("Error")]
    assert "HnmError" in exceptions
    assert "AuxGpioRejectedError" in exceptions
    assert len(exceptions) == 21
    for name in exceptions:
        exception: object = getattr(hnm, name)
        assert isinstance(exception, type)
        assert issubclass(exception, HnmError) or exception is HnmError
    assert issubclass(HnmError, Exception)


def test_node_types_metadata() -> None:
    assert hnm.NodeType.GATEWAY.identifier == "hnm.gateway"
    assert hnm.NodeType.GATEWAY.label == "Gateway"
    assert hnm.NodeType.GATEWAY.description
    assert hnm.NodeType.SIDECAR != hnm.NodeType.ROTATOR
    assert not hasattr(hnm, "Node")
    assert not hasattr(hnm, "Gateway")


# === Private Functions === #

# === Entry Point === #
