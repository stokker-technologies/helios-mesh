# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Exact Commit Validation Before Stable Tagging."""

# === Imports === #

# Standard Library Imports
import copy
import runpy
import unittest
from pathlib import Path

# Third Party Imports

# Local Imports

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[4]

# === Types === #

# === Classes === #


class ReleaseGateTests(unittest.TestCase):
    def test_exact_gate(self):
        guard = runpy.run_path(str(ROOT / ".github/actions/review-release/checks.py"))
        sha = "a" * 40
        run = {"id": 8, "head_sha": sha, "head_branch": "dev", "event": "push", "status": "in_progress", "check_suite_id": 9}
        gate = {
            "id": 10,
            "name": "CI / Required",
            "app": {"slug": "github-actions", "id": 1},
            "check_suite": {"id": 9},
            "status": "completed",
            "conclusion": "success",
        }
        checks = [gate]
        rules = []

        def pages(endpoint, key=None):
            if endpoint.startswith("actions/"):
                return [run]
            if "/check-runs?" in endpoint:
                return checks
            if endpoint == "rules/branches/main":
                return rules
            if endpoint.endswith("/statuses"):
                return []
            raise AssertionError(endpoint)

        self.assertTrue(guard["green"](sha, pages))
        run["head_sha"] = "b" * 40
        self.assertFalse(guard["green"](sha, pages))
        run["head_sha"] = sha
        gate["check_suite"]["id"] = 11
        self.assertFalse(guard["green"](sha, pages))
        gate["check_suite"]["id"] = 9
        rules.append({"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "Native / Windows"}]}})
        self.assertFalse(guard["green"](sha, pages))
        native = copy.deepcopy(gate)
        native.update(id=12, name="Native / Windows", conclusion="failure")
        checks.append(native)
        with self.assertRaises(ValueError):
            guard["green"](sha, pages)
        native["conclusion"] = "success"
        self.assertTrue(guard["green"](sha, pages))


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
