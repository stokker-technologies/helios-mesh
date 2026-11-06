# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Explicit Coverage Selection Replaces Default Source Roots."""

# === Imports === #

# Standard Library Imports
import tempfile
import sys
import unittest
from pathlib import Path
from unittest.mock import patch
from xml.etree import ElementTree

# Third Party Imports

# Local Imports
from helios_mesh_tools.test import report, run

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #


class CoverageTests(unittest.TestCase):
    def test_selected_module(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "tools").mkdir()
            (root / "tools/project.toml").write_text("[conventions.testing]\n")
            (root / "pyproject.toml").write_text('[project]\nname="coverage-fixture"\nversion="0.0.0"\n')
            (root / "src").mkdir()
            (root / "src/unused.py").write_text("def unused():\n    return 0\n")
            (root / "sample.py").write_text("def increment(value):\n    return value + 1\n")
            (root / "tests").mkdir()
            (root / "tests/test_sample.py").write_text(
                "from sample import increment\n\ndef test_increment():\n    assert increment(2) == 3\n"
            )
            with (
                patch.object(sys, "stdin", sys.__stdin__),
                patch.object(sys, "stdout", sys.__stdout__),
                patch.object(sys, "stderr", sys.__stderr__),
            ):
                self.assertEqual(run.coverage(root, "python", None, ".", "dev", ["--cov-reset", "--cov=sample"]), 0)
            document = ElementTree.parse(report.location(root, "dev", "coverage-python") / "coverage.xml")
            classes = document.findall(".//class")
            self.assertEqual([Path(item.attrib["filename"]).name for item in classes], ["sample.py"])
            self.assertGreater(float(classes[0].attrib["line-rate"]), 0)


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
