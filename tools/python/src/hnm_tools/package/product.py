# ***
#
# Helios Node Mesh
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
from ..test import archive

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
    wheels = sorted((ROOT / "target/wheels").glob("*.whl"))
    if not wheels:
        raise archive.policy.TestError("missing native wheels; run the build task first")
    required = [path.relative_to(ROOT).as_posix() for path in wheels]
    if options.language == "rust":
        required.append("target/fixtures/bin/hnm{exe}")
        if platform.system() == "Linux":
            required.extend(f"target/fixtures/firmware/hnm-node-{node}" for node in ("gateway", "rotator", "sidecar"))
    archive.fixtures(ROOT, options.profile, required)

    # Keep Each Native Wheel In Its Platform's Package Artifact.
    destination = ROOT / "target/package/python"
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    for wheel in wheels:
        shutil.copy2(wheel, destination / wheel.name)
    if options.language == "python":
        return

    executable = "hnm.exe" if os.name == "nt" else "hnm"
    source = ROOT / "target/fixtures/bin" / executable
    native = ROOT / "target/package/native"
    native.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, native / executable)

    if platform.system() == "Linux":
        firmware = ROOT / "target/package/firmware"
        firmware.mkdir(parents=True, exist_ok=True)
        for node in ("gateway", "rotator", "sidecar"):
            source = ROOT / "target/fixtures/firmware" / f"hnm-node-{node}"
            subprocess.run(["elf2uf2-rs", str(source), str(firmware / f"hnm-node-{node}.uf2")], cwd=ROOT, check=True)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
