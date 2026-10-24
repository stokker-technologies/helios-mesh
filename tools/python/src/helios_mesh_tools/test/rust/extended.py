# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Build And Run The Local Instrumented Transport Suite"""

# === Imports === #

# Standard Library Imports
import argparse
import subprocess
import sys
from pathlib import Path

# Third Party Imports
# Local Imports
from .. import nextest, policy
from ...build import cargo, paths

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
    execute(options.profile)


def execute(profile: str) -> None:
    compiler_profile = "test"
    arguments = ["--locked", "--package", "helios-mesh", "--test", "transport", "--features", "testing"]
    cargo.clippy(ROOT, [*arguments, "--profile", compiler_profile])
    output = paths.output(ROOT, "tests", profile) / "extended-rust"
    output.mkdir(parents=True, exist_ok=True)
    configuration = output / "nextest.toml"
    rules = policy.load(ROOT)
    overrides = nextest.overrides(rules, "integration", {}, False, {})
    configuration.write_text(nextest.config(str(output / "store"), rules.deadline("integration"), overrides), encoding="utf-8")
    subprocess.run(
        [
            "cargo",
            "nextest",
            "run",
            *arguments,
            "--cargo-profile",
            compiler_profile,
            "--config-file",
            str(configuration),
            "--profile",
            nextest.PROFILE,
        ],
        cwd=ROOT,
        stdin=sys.stdin,
        stdout=sys.stdout,
        stderr=sys.stderr,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        check=True,
    )


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
