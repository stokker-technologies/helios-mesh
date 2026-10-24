# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Check Noncanonical Rust Feature Combinations Locally"""

# === Imports === #

# Standard Library Imports
import argparse
from pathlib import Path

# Third Party Imports

# Local Imports
from .. import cargo

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
    check(options.profile)


# === Private Functions === #


def check(profile: str) -> None:
    compiler_profile = "release-tests" if profile == "release" else profile
    configurations = [
        ["--workspace", "--exclude", "helios-mesh-nodes", "--all-targets", "--all-features"],
        ["--workspace", "--exclude", "helios-mesh-nodes", "--all-targets", "--no-default-features"],
        ["--package", "helios-mesh", "--all-targets", "--all-features"],
        ["--package", "helios-mesh", "--all-targets", "--no-default-features"],
        ["--package", "helios-mesh-nodes", "--lib", "--all-features"],
        ["--package", "helios-mesh-nodes", "--bins", "--target", "thumbv6m-none-eabi", "--all-features"],
    ]
    for configuration in configurations:
        cargo.clippy(ROOT, ["--locked", "--profile", compiler_profile, *configuration])


# === Entry Point === #

if __name__ == "__main__":
    main()
