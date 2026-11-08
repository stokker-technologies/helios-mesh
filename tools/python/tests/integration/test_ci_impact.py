# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Conservative Impact Selection With The Simulation's Native Plan."""

# === Imports === #

# Standard Library Imports
import unittest
from pathlib import Path
from unittest.mock import patch

# Third Party Imports

# Local Imports
from helios_mesh_tools.ci import impact
from helios_mesh_tools.ci.simulate import PLAN

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #


class ImpactSelectionTests(unittest.TestCase):
    def test_branch_changes_use_native_rules_and_dependency_closure(self):
        plan = PLAN["impact"]
        full = set(plan["dependencies"])
        cases = [
            (None, full),
            (set(), full),
            ({"tools/python/src/helios_mesh_tools/ci/simulate.py"}, full),
            ({"README.md", "docs/guide.md"}, {"check:repository"}),
            ({"LICENSE"}, {"package:rust", "build", "check:python", "check:python:generated", "check:repository", "check:rust"}),
            ({"docs/examples/example.md"}, full),
            ({"README.md", "crates/system/helios-mesh/src/lib.rs"}, full),
        ]
        for paths, expected in cases:
            with self.subTest(paths=paths), patch.object(impact, "changed_paths", return_value=paths):
                self.assertEqual(impact.selection(Path.cwd(), plan), expected)

    def test_protected_paths_override_matching_rules(self):
        plan = dict(PLAN["impact"], rules=[{"only": [".*"], "exclude": [], "jobs": []}])
        self.assertEqual(impact.select(plan, {"docs/guide.md"}), set())
        for path in (".github/workflows/ci.yml", "tools/python/pyproject.toml", "mise.toml"):
            with self.subTest(path=path):
                self.assertEqual(impact.select(plan, {path}), set(plan["dependencies"]))


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
