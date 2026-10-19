# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Assemble CLI, Firmware And Wheel Packages From The Selected Build."""

# === Imports === #

# Standard Library Imports
import argparse
import os
import platform
import shutil
import subprocess
from pathlib import Path

# Third Party Imports

# Local Imports
from .. import temporary
from ..build import paths

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    parser.add_argument("--language", choices=("rust", "python"), required=True)
    options = parser.parse_args()
    product = paths.output(ROOT, "build", options.profile)
    destination = paths.output(ROOT, "package", options.profile) / options.language
    if options.language == "python":
        if not any((product / "python").glob("*.whl")):
            raise ValueError("missing native wheels; run the matching Product Build first")
        paths.replace(ROOT, product / "python", destination)
        return

    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    with temporary.directory() as staging:
        shutil.copy2(product / "bin" / executable, staging / executable)
        if platform.system() == "Linux":
            firmware = staging / "firmware"
            firmware.mkdir()
            for node in ("gateway", "rotator", "sidecar"):
                source = product / "firmware" / f"helios-mesh-node-{node}"
                subprocess.run(["elf2uf2-rs", str(source), str(firmware / f"helios-mesh-node-{node}.uf2")], cwd=ROOT, check=True)
        paths.replace(ROOT, staging, destination, origin=staging)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
