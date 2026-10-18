# ***
#
# Helios Node Mesh
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
from . import archive

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

    wheels = sorted((ROOT / "target/wheels").glob("*.whl"))
    if not wheels:
        raise archive.policy.TestError("missing native wheels; run the build task first")
    required = [path.relative_to(ROOT).as_posix() for path in wheels]
    archive.fixtures(ROOT, options.profile, required)
    archive.install_wheels(ROOT)

    subprocess.run([sys.executable, "-m", "mypy.stubtest", "--ignore-disjoint-bases", "hnm"], cwd=ROOT, check=True)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
