# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Append Completed Suite Results To Their Published Nightly."""

# === Imports === #

# Standard Library Imports
import json
import os
from pathlib import Path

# Third Party Imports

# Local Imports
from .. import policy
from ..release import registry

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def report(directory: Path, results: dict) -> None:
    # Read The Published Nightly And Its Source.
    settings = policy.policy(directory, "release")
    release = registry.document(settings, "releases/tags/nightly", channel=True)
    sha = os.environ["GITHUB_SHA"]
    identifier = os.environ["GITHUB_RUN_ID"]
    if not release or registry.remote_tag(settings, "nightly") != sha:
        return
    body = release.get("body", "")
    if f"| CI Run | [{identifier}]" not in body:
        return

    # Append Every Suite Outcome And The Pipeline URL.
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
    repository = os.environ["GITHUB_REPOSITORY"]
    url = f"{server}/{repository}/actions/runs/{identifier}"
    lines = ["", "### Suite Results", "", "| Suite | Result |", "| --- | --- |"]
    lines.extend(f"| {name.replace('_', ' ').title()} | {job['result']} |" for name, job in results.items())
    lines.extend(["", f"[Pipeline Results]({url})", ""])
    registry.document(settings, f"releases/{release['id']}", "PATCH", {"body": body + "\n".join(lines)}, channel=True)


def main() -> None:
    report(Path.cwd(), json.loads(os.environ["RESULTS"]))


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    main()
