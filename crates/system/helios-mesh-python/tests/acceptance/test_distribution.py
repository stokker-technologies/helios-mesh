# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Verify The Installed Wheel, Native Extension And Typing Marker."""

# === Imports === #

# Standard Library Imports
import re
import sys
from importlib import metadata
from pathlib import Path

# Third Party Imports
import helios_mesh
from helios_mesh import _native

# Local Imports

# Type Checking Imports


# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]

# === Types === #

# === Classes === #

# === Public Functions === #


def test_installed_distribution() -> None:
    distribution = metadata.distribution("helios-mesh")
    workspace = (ROOT / "Cargo.toml").read_text().split("[workspace.package]", 1)[1]
    package = workspace.split("\n[", 1)[0]
    match = re.search(r'^version = "([^"]+)"$', package, re.MULTILINE)
    assert match is not None
    expected = match.group(1)
    assert distribution.version == expected
    assert distribution.metadata["License-Expression"] == "Apache-2.0"
    assert Path(helios_mesh.__file__).resolve().is_relative_to(Path(sys.prefix).resolve())
    assert any(str(path) == "helios_mesh/py.typed" for path in distribution.files or ())
    assert _native.__file__.endswith((".so", ".pyd"))


# === Private Functions === #

# === Entry Point === #
