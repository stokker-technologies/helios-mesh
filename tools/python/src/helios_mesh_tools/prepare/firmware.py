# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Install The Pinned Firmware Converter After Compiler Output Preflight."""

# === Imports === #

# Standard Library Imports
import os
import subprocess
import sys
from pathlib import Path

# Third Party Imports

# Local Imports
from ..build import storage
from ..process import spawn_flags

# Type Checking Imports


# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #

# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__" and sys.platform == "linux":
    root = Path.cwd()
    target = root / "target/tooling/firmware-converter"
    storage.preflight(root, target=target)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target))
    environment.pop("UV_PROJECT_ENVIRONMENT", None)
    subprocess.run(["mise", "install", "--yes", sys.argv[1]], cwd=root, env=environment, creationflags=spawn_flags(), check=True)
