# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Prove Native Installation Roots And Scratch Isolation."""

# === Imports === #

# Standard Library Imports
import runpy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# Third Party Imports

# Local Imports

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[4]

# === Types === #

# === Classes === #


class SetupTests(unittest.TestCase):
    def test_installation_roots(self):
        native = runpy.run_path(str(ROOT / ".github/actions/setup/run.py"))
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            workspace, home, scratch = base / "checkout", base / "home", base / "scratch"
            with patch.object(native["tempfile"], "gettempdir", return_value=str(scratch)):
                windows = native["roots"]({}, workspace, home, "windows")
                unix = native["roots"]({}, workspace, home, "unix")
                configured = native["roots"]({"MISE_DATA_DIR": str(scratch / "custom")}, workspace, home, "windows")
                self.assertEqual(Path(windows["MISE_DATA_DIR"]), scratch / "mise")
                self.assertEqual(Path(unix["MISE_DATA_DIR"]), home / ".local/share/mise")
                self.assertEqual(Path(configured["MISE_DATA_DIR"]), scratch / "custom")
                self.assertEqual(windows["WORKSPACE_ROOT_ID"], unix["WORKSPACE_ROOT_ID"])
                self.assertNotEqual(windows["MISE_ROOT_ID"], unix["MISE_ROOT_ID"])
            with patch.object(native["tempfile"], "gettempdir", return_value=str(workspace / "scratch")):
                with self.assertRaises(RuntimeError):
                    native["temporary_root"](workspace)


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
