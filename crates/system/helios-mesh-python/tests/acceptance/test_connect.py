# ***
#
# Helios Mesh
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
import helios_mesh
from helios_mesh import DeviceNotFoundError, GatewayNode, HeliosMeshError, NodeHandle
from helios_mesh.node import DeviceSelector, NodeId

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
    exceptions = [name for name in helios_mesh.__all__ if name.endswith("Error")]
    assert "HeliosMeshError" in exceptions
    assert "AuxGpioRejectedError" in exceptions
    assert len(exceptions) == 21
    for name in exceptions:
        exception: object = getattr(helios_mesh, name)
        assert isinstance(exception, type)
        assert issubclass(exception, HeliosMeshError) or exception is HeliosMeshError
    assert issubclass(HeliosMeshError, Exception)


def test_node_types_metadata() -> None:
    assert helios_mesh.NodeType.GATEWAY.identifier == "hnm.gateway"
    assert helios_mesh.NodeType.GATEWAY.label == "Gateway"
    assert helios_mesh.NodeType.GATEWAY.description
    assert helios_mesh.NodeType.SIDECAR != helios_mesh.NodeType.ROTATOR
    assert not hasattr(helios_mesh, "Node")
    assert not hasattr(helios_mesh, "Gateway")


# === Private Functions === #

# === Entry Point === #
