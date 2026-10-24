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
    compiler_profile = "test"
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


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
