# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Build The Shared Test Archive From Matching Product Outputs"""

# === Imports === #

# Standard Library Imports
import argparse
import os
from pathlib import Path

# Third Party Imports

# Local Imports
from ..test import archive
from . import paths

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
    options = parser.parse_args()
    output = paths.output(ROOT, "build", options.profile)
    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    host = output / "bin" / executable
    wheels = sorted((output / "python").glob("*.whl"))
    if not host.is_file() or not wheels:
        raise ValueError("missing matching Product Build; run mise run build first")

    archive.build(ROOT, options.profile, ("--exclude", "helios-mesh-python", "--features", "helios-mesh/testing"))
    files = [host.relative_to(ROOT).as_posix(), *(wheel.relative_to(ROOT).as_posix() for wheel in wheels)]
    archive.stamp(ROOT, options.profile, files)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
