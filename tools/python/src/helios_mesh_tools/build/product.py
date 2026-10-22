# ***
#
# Helios Mesh
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
from pathlib import Path

# Third Party Imports
# Local Imports
from .. import temporary
from . import paths, run

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
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    parser.add_argument("--hardware-unknown", action="store_true")
    options = parser.parse_args()
    profile = options.profile
    if options.hardware_unknown and profile != "dev":
        parser.error("hardware diagnostics use the development profile")

    destination = paths.output(ROOT, "build", profile)
    messages = destination / "product/compiler.json"
    messages.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "cargo",
        "build",
        "--locked",
        "--profile",
        profile,
        "--package",
        "helios-mesh-cli",
        "--message-format=json-render-diagnostics",
    ]
    with messages.open("w", encoding="utf-8") as output:
        subprocess.run(command, cwd=ROOT, stdout=output, check=True)
    executable = "helios-mesh.exe" if os.name == "nt" else "helios-mesh"
    with temporary.directory() as staging:
        binary = None
        for line in messages.read_text(encoding="utf-8").splitlines():
            event = json.loads(line)
            if event.get("reason") == "compiler-artifact" and event.get("target", {}).get("name") == "helios-mesh":
                binary = event.get("executable") or binary
        if binary is None:
            raise ValueError("Product Build did not produce the native CLI")
        shutil.copy2(binary, staging / executable)
        paths.replace(ROOT, staging, destination / "bin", origin=staging)
    run.record_target(ROOT, messages, profile)

    wheels = destination / "python"
    if wheels.exists():
        shutil.rmtree(wheels)
    wheels.mkdir(parents=True)
    command = [
        "maturin",
        "build",
        "--locked",
        "--profile",
        profile,
        "--manifest-path",
        "crates/system/helios-mesh-python/Cargo.toml",
        "--out",
        str(wheels),
    ]
    subprocess.run(command, cwd=ROOT, check=True)

    if platform.system() == "Linux":
        command = [
            "cargo",
            "build",
            "--locked",
            "--profile",
            profile,
            "--target",
            TARGET,
            "--package",
            "helios-mesh-nodes",
            "--features",
            "firmware",
        ]
        if options.hardware_unknown:
            command[-1] = "firmware,hardware-unknown"
        subprocess.run(command, cwd=ROOT, check=True)
        directory = "debug" if profile == "dev" else profile
        with temporary.directory() as staging:
            for node in ("gateway", "rotator", "sidecar"):
                source = ROOT / "target" / TARGET / directory / f"helios-mesh-node-{node}"
                shutil.copy2(source, staging / source.name)
            paths.replace(ROOT, staging, destination / "firmware", origin=staging)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
