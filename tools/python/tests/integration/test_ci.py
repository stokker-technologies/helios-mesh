# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Impact Selection, Required Results And Exact Release Evidence."""

# === Imports === #

# Standard Library Imports
import copy
import json
import os
import runpy
import unittest
from pathlib import Path
from unittest.mock import patch

# Third Party Imports

# Local Imports
from helios_mesh_tools.ci import nightly
from helios_mesh_tools.package import product
from helios_mesh_tools import policy
from helios_mesh_tools.release import registry

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[4]

# === Types === #

# === Classes === #


class ImpactTests(unittest.TestCase):
    def test_scope_keeps_only_quick_checks_and_product(self):
        action = runpy.run_path(str(ROOT / ".github/actions/plan-impact/run.py"))
        selected = {"check", "build_product", "build_tests", "test", "package"}
        self.assertEqual(action["scope"]({}, selected, "build"), {"check", "build_product"})
        self.assertEqual(action["scope"]({}, selected), selected)

    def test_selected_jobs_cannot_silently_skip(self):
        action = runpy.run_path(str(ROOT / ".github/actions/plan-impact/gate.py"))
        results = {
            "impact": {"result": "success", "outputs": {"jobs": json.dumps(["check", "build_product"])}},
            "check": {"result": "success"},
            "build_product": {"result": "success"},
            "test": {"result": "skipped"},
        }
        action["validate"](results)
        for result in ("skipped", "failure", "cancelled"):
            changed = copy.deepcopy(results)
            changed["build_product"]["result"] = result
            with self.subTest(result=result), self.assertRaises(ValueError):
                action["validate"](changed)
        del results["build_product"]
        with self.assertRaises(ValueError):
            action["validate"](results)


class WheelInventoryTests(unittest.TestCase):
    def test_complete_shared_version_and_portable_tags(self):
        names = [
            "helios_mesh-0.1.5-cp310-abi3-manylinux_2_39_x86_64.whl",
            "helios_mesh-0.1.5-cp310-abi3-win_amd64.whl",
        ]
        targets = ["linux-x86_64", "windows-x86_64"]
        self.assertEqual(product.wheel_inventory(["SHA256SUMS", *names], targets), dict(zip(targets, names, strict=True)))
        for invalid in (
            names[:-1],
            [*names, names[0]],
            [names[0], names[1].replace("0.1.5", "0.1.4")],
            [names[0].replace("manylinux_2_39", "linux"), names[1]],
            [names[0], names[1].replace("cp310-abi3", "cp315-cp315")],
        ):
            with self.subTest(names=invalid), self.assertRaises(ValueError):
                product.wheel_inventory(invalid, targets)


class ReleaseEvidenceTests(unittest.TestCase):
    def test_rehearsal_requires_the_merged_revision_and_success(self):
        sha = "a" * 40
        runs = [{"id": 8, "head_sha": sha, "head_branch": "dev", "event": "push"}]
        jobs = [{"name": "Release / Rehearsal", "conclusion": "success"}]

        def pages(settings, endpoint, key):
            if endpoint == f"actions/workflows/release.yml/runs?branch=dev&head_sha={sha}":
                return runs
            self.assertEqual(endpoint, "actions/runs/8/jobs?filter=latest")
            return jobs

        self.enterContext(patch.object(registry, "pages", side_effect=pages))
        registry.rehearsal({}, sha)
        jobs[0]["conclusion"] = "failure"
        with self.assertRaises(policy.ToolError):
            registry.rehearsal({}, sha)
        jobs[0]["conclusion"] = "success"
        runs[0]["head_sha"] = "b" * 40
        with self.assertRaises(policy.ToolError):
            registry.rehearsal({}, sha)

    def test_report_updates_only_the_publishing_run(self):
        sha = "a" * 40
        release = {"id": 4, "body": "| CI Run | [42](https://github.com/owner/repo/actions/runs/42) |\n"}
        environment = {"GITHUB_SHA": sha, "GITHUB_RUN_ID": "42", "GITHUB_REPOSITORY": "owner/repo"}
        self.enterContext(patch.dict(os.environ, environment))
        self.enterContext(patch.object(nightly.policy, "policy", return_value={}))
        self.enterContext(patch.object(nightly.registry, "remote_tag", return_value=sha))
        document = self.enterContext(patch.object(nightly.registry, "document", return_value=release))
        nightly.report(ROOT, {"test": {"result": "failure"}, "coverage": {"result": "success"}})
        self.assertEqual(document.call_count, 2)
        self.assertEqual(document.call_args.args[1:3], ("releases/4", "PATCH"))
        document.reset_mock()
        release["body"] = "| CI Run | [43](https://github.com/owner/repo/actions/runs/43) |\n"
        nightly.report(ROOT, {"test": {"result": "success"}})
        document.assert_called_once()


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
