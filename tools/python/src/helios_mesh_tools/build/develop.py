# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Run Development Interfaces With Compiler Storage Preflight."""

# === Imports === #

# Standard Library Imports
import subprocess
import sys
from pathlib import Path

# Third Party Imports
# Local Imports
from ..process import spawn_flags
from . import cargo, storage

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #

# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    root = Path.cwd()
    if sys.argv[1] == "cli":
        raise SystemExit(cargo.focused(root, "run", ["--locked", "--package", "helios-mesh-cli", "--", *sys.argv[2:]]))
    storage.preflight(root)
    raise SystemExit(
        subprocess.run(
            ["maturin", "develop", "--manifest-path", "crates/system/helios-mesh-python/Cargo.toml", "--uv", *sys.argv[2:]],
            creationflags=spawn_flags(),
            check=False,
        ).returncode
    )
