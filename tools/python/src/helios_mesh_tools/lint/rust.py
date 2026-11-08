# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Lint The Shipping Rust Selections With Clippy"""

# === Imports === #

# Standard Library Imports
import tomllib
from pathlib import Path

# Third Party Imports
# Local Imports
from ..build import cargo

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[5]
TARGET = "thumbv6m-none-eabi"

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    cargo.clippy(ROOT, ["--locked", "--workspace", "--exclude", "helios-mesh-python", "--all-targets"])
    with (ROOT / "crates/system/helios-mesh-python/pyproject.toml").open("rb") as source:
        features = tomllib.load(source)["tool"]["maturin"]["features"]
    arguments = ["--locked", "--package", "helios-mesh-python", "--lib"]
    if features:
        arguments.extend(("--features", ",".join(features)))
    cargo.clippy(ROOT, arguments)
    cargo.clippy(ROOT, ["--locked", "--package", "helios-mesh-nodes", "--bins", "--target", TARGET, "--features", "firmware"])


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
