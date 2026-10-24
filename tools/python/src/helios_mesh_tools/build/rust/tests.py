# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Build The Independent Canonical Rust Test Archive"""

# === Imports === #

# Standard Library Imports
import argparse
from pathlib import Path

# Third Party Imports
# Local Imports
from ...test import archive
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
    built = cargo.archive(ROOT, options.profile, ("--exclude", "helios-mesh-python"))
    files = [path.relative_to(ROOT).as_posix() for path in (built, built.with_name(archive.MESSAGES))]
    archive.stamp(ROOT, options.profile, files)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
