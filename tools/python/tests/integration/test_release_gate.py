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
import json
import os
import runpy
import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# Third Party Imports

# Local Imports
from helios_mesh_tools import policy
from helios_mesh_tools.release import registry
from helios_mesh_tools.screenshots import run

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[4]

# === Types === #

# === Classes === #


class ReleaseGateTests(unittest.TestCase):
    def test_screenshot_contract(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        folder = root / "screenshots"
        folder.mkdir()
        image = folder / "cli-idle.png"
        data = b"\x89PNG\r\n\x1a\n" + struct.pack(">I4sII", 13, b"IHDR", 640, 480) + bytes(9)
        image.write_bytes(data)
        (folder / "README.md").write_text("![CLI](cli-idle.png)", encoding="utf-8")
        entry = {
            "image": image.name,
            "surface": "cli",
            "state": "idle",
            "viewport": {"width": 640, "height": 480},
            "revision": "a" * 40,
            "captured_at": "2026-01-01T00:00:00+00:00",
        }
        (folder / "manifest.json").write_text(json.dumps([entry]), encoding="utf-8")
        settings = {"integration_branch": "dev", "max_commits_behind": 2}
        self.enterContext(patch.object(run.policy, "policy", return_value=settings))
        execute = self.enterContext(patch.object(run.policy, "execute", side_effect=["", "2"]))
        run.check(root)
        self.assertEqual(execute.call_args_list[0].args[1], ["git", "merge-base", "--is-ancestor", entry["revision"], "dev"])
        self.assertEqual(execute.call_args_list[1].args[1], ["git", "rev-list", "--count", entry["revision"] + "..dev"])
        execute.side_effect = ["", "3"]
        with self.assertRaises(policy.ToolError):
            run.check(root)
        execute.reset_mock()
        image.write_bytes(data + bytes(5 * 1024 * 1024))
        with self.assertRaises(policy.ToolError):
            run.check(root)
        execute.assert_not_called()

    def test_wheel_profile(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for profile in ("dev", "release"):
            wheel = root / f"target/package/{profile}/native/python/example.whl"
            wheel.parent.mkdir(parents=True)
            wheel.write_bytes(profile.encode())
        self.enterContext(patch.dict(os.environ, {"PROFILE": "dev"}, clear=True))
        execute = self.enterContext(patch.object(registry.policy, "execute"))
        registry.python(root, {"provider": "github"}, dry_run=True)
        self.assertEqual(Path(execute.call_args.args[1][-1]), Path("target/package/dev/native/python/example.whl"))
        with patch.dict(os.environ, {"RELEASE_PROFILE": "release"}):
            registry.python(root, {"provider": "github"}, dry_run=True)
            self.assertEqual(Path(execute.call_args.args[1][-1]), Path("target/package/release/native/python/example.whl"))

    def test_exact_gate(self) -> None:
        guard = runpy.run_path(str(ROOT / ".github/actions/review-release/checks.py"))
        sha = "a" * 40
        run = {"id": 8, "head_sha": sha, "head_branch": "dev", "event": "push", "status": "in_progress", "check_suite_id": 9}
        gate = {
            "id": 10,
            "name": "Check / CI / Required",
            "app": {"slug": "github-actions", "id": 1},
            "check_suite": {"id": 9},
            "status": "completed",
            "conclusion": "success",
        }
        checks = [gate]
        rules = []

        def pages(endpoint: str, key: str | None = None) -> list[dict]:
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
