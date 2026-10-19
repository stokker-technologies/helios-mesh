# Rendered by stkr project update from templates/python/package/run.py; sha256:a34eaef4a8b56ecfbae7bc644ae725c0bdbd243240de7d0390a626f6f206815a
# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Collect Packages Under Their Matching Profile And Native Target"""

# === Imports === #

# Standard Library Imports
import argparse
import subprocess
from pathlib import Path

# Third Party Imports
# Local Imports
from .. import temporary
from ..build import paths

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def main() -> None:
    parser = argparse.ArgumentParser(description="Assemble Existing Build Outputs")
    parser.add_argument("language", choices=("rust", "python", "typescript", "native"))
    parser.add_argument("--profile", choices=("dev", "release"), default="dev")
    parser.add_argument("--target")
    options = parser.parse_args()
    directory = Path.cwd()
    destination = paths.output(directory, "package", options.profile, options.target)
    if options.language == "rust":
        with temporary.directory() as scratch:
            subprocess.run(
                [
                    "cargo",
                    "package",
                    "--workspace",
                    "--locked",
                    "--no-verify",
                    "--target-dir",
                    str(scratch),
                ],
                cwd=directory,
                check=True,
            )
            paths.replace(
                directory,
                scratch / "package",
                destination / options.language,
                origin=scratch,
            )
        return
    else:
        kind = {"python": "python", "typescript": "web", "native": "bin"}[options.language]
        source = paths.output(directory, "build", options.profile, options.target) / kind
    paths.replace(directory, source, destination / options.language)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
