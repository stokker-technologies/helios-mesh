# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Build Native Host Artifacts, Wheels And Linux Firmware For One Profile."""

# === Imports === #

# Standard Library Imports
import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

# Third Party Imports
# Local Imports
from ..test import archive

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]
TARGET = "thumbv6m-none-eabi"

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("dev", "release", "bench"), default="dev")
    parser.add_argument("--hardware-unknown", action="store_true")
    options = parser.parse_args()
    profile = options.profile
    if options.hardware_unknown and profile != "dev":
        parser.error("hardware diagnostics use the development profile")

    archive.build(ROOT, profile, ("--exclude", "hnm-python", "--features", "hnm/testing"))
    stamp = json.loads((ROOT / archive.FIXTURES).read_text())
    files = list(stamp["files"])
    wheels = ROOT / "target/wheels"
    wheels.mkdir(parents=True, exist_ok=True)
    for old in wheels.glob("*.whl"):
        old.unlink()
    command = [
        sys.executable,
        "-m",
        "maturin",
        "build",
        "--locked",
        "--profile",
        profile,
        "--manifest-path",
        "crates/system/hnm-python/Cargo.toml",
        "--out",
        str(wheels),
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    archive.install_wheels(ROOT)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    smoke = f"import hnm; from importlib.metadata import version; assert version('hnm') == {version!r}; assert hnm.NodeType.SIDECAR.identifier == 'hnm.sidecar'"
    subprocess.run([sys.executable, "-c", smoke], cwd=ROOT, check=True)

    if platform.system() == "Linux" and profile != "bench":
        command = [
            "cargo",
            "build",
            "--locked",
            "--profile",
            profile,
            "--target",
            TARGET,
            "--package",
            "hnm-nodes",
            "--features",
            "firmware",
        ]
        if options.hardware_unknown:
            command[-1] = "firmware,hardware-unknown"
        subprocess.run(command, cwd=ROOT, check=True)
    directory = "debug" if profile == "dev" else profile
    executable = "hnm.exe" if os.name == "nt" else "hnm"
    host = f"target/fixtures/bin/{executable}"
    if host not in files:
        raise archive.policy.TestError("nextest did not produce the native CLI fixture")
    files.extend(path.relative_to(ROOT).as_posix() for path in wheels.glob("*.whl"))
    if platform.system() == "Linux" and profile != "bench":
        firmware = ROOT / "target/fixtures/firmware"
        firmware.mkdir(parents=True, exist_ok=True)
        for node in ("gateway", "rotator", "sidecar"):
            source = ROOT / "target" / TARGET / directory / f"hnm-node-{node}"
            destination = firmware / source.name
            shutil.copy2(source, destination)
            files.append(destination.relative_to(ROOT).as_posix())
    archive.stamp(ROOT, profile, files)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
