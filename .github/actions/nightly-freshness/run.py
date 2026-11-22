# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Skip Only A Complete Published Nightly Of The Current Commit."""

# === Imports === #

# Standard Library Imports
import json
import os
import subprocess
from pathlib import Path

# Third Party Imports

# Local Imports
from helios_mesh_tools.package.product import wheel_inventory

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def fresh(release: dict, tagged: str, sha: str) -> bool:
    if release.get("draft", True) or not release.get("prerelease") or tagged != sha:
        return False
    if f"/commit/{sha})" not in release.get("body", ""):
        return False
    assets = {
        item["name"]
        for item in release.get("assets", [])
        if item.get("state") == "uploaded" and item.get("size", 0) > 0
    }
    if not {"CHANGELOG.md", "SHA256SUMS"} <= assets:
        return False
    try:
        wheel_inventory(sorted(assets), ["linux-x86_64", "windows-x86_64"])
    except ValueError:
        return False
    return True


def main() -> None:
    # Read The Published Release; A Tag Alone Does Not Establish Publication.
    repository = os.environ["GITHUB_REPOSITORY"]
    result = subprocess.run(
        ["gh", "api", f"repos/{repository}/releases/tags/nightly"],
        capture_output=True,
        text=True,
        check=False,
    )
    build = True
    if result.returncode and "HTTP 404" not in result.stderr:
        raise RuntimeError(
            f"restore access to the nightly release before retrying: {result.stderr}"
        )
    if result.returncode == 0:
        release = json.loads(result.stdout)
        result = subprocess.run(
            ["gh", "api", f"repos/{repository}/commits/nightly"],
            capture_output=True,
            text=True,
            check=True,
        )
        tagged = json.loads(result.stdout)["sha"]
        build = not fresh(release, tagged, os.environ["GITHUB_SHA"])

    # Workflow Concurrency Serializes Duplicate Runs; A Queued Run Cannot Suppress This One.
    destination = Path(os.environ["GITHUB_OUTPUT"])
    destination.write_text(f"build={str(build).lower()}\n", encoding="utf-8")
    print("Build Nightly" if build else "Published Nightly Is Current")


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
