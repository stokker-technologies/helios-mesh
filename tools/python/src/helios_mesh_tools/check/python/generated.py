# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Verify The Selected Native Wheel Against Its Python Stubs"""

# === Imports === #

# Standard Library Imports
import argparse
import subprocess
import sys
from pathlib import Path

# Third Party Imports

# Local Imports
from ...test import archive
from ...build import paths

# Type Checking Imports


# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[6]

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    options = parser.parse_args()

    output = paths.output(ROOT, "build", options.profile)
    wheels = sorted((output / "python").glob("*.whl"))
    if not wheels:
        raise archive.policy.TestError("missing native wheels; run the build task first")
    archive.install_wheels(ROOT, options.profile)

    subprocess.run(
        [sys.executable, "-m", "mypy.stubtest", "--ignore-disjoint-bases", "helios_mesh"],
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        stdin=sys.stdin,
        stdout=sys.stdout,
        stderr=sys.stderr,
        cwd=ROOT,
        check=True,
    )


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
