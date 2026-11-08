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
from email.message import Message
import hashlib
import io
import runpy
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import patch

# Third Party Imports

# Local Imports

# Type Checking Imports

# === Public Exports === #

# === Constants === #

ROOT = Path(__file__).resolve().parents[4]
URL = "https://example.com/archive.zip"
PAYLOAD = b"archive"

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

    def test_download_retries_transient_failure(self):
        native = runpy.run_path(str(ROOT / ".github/actions/setup/run.py"))
        responses = [urllib.error.HTTPError(URL, 503, "unavailable", Message(), None), io.BytesIO(PAYLOAD)]
        calls, waits = [], []

        def opener(url, timeout):
            calls.append(url)
            response = responses.pop(0)
            if isinstance(response, Exception):
                raise response
            return response

        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "cache/archive.zip"
            native["download"](URL, hashlib.sha256(PAYLOAD).hexdigest(), destination, opener=opener, sleep=waits.append)
            self.assertEqual(destination.read_bytes(), PAYLOAD)
            self.assertEqual(list(destination.parent.iterdir()), [destination])
        self.assertEqual(len(calls), 2)
        self.assertEqual(len(waits), 1)

    def test_download_does_not_retry_client_error(self):
        native = runpy.run_path(str(ROOT / ".github/actions/setup/run.py"))
        calls = []

        def opener(url, timeout):
            calls.append(url)
            raise urllib.error.HTTPError(url, 404, "missing", Message(), None)

        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(RuntimeError):
                native["download"](URL, "0" * 64, Path(temporary) / "archive.zip", opener=opener, sleep=self.fail)
        self.assertEqual(len(calls), 1)

    def test_download_does_not_retry_checksum_mismatch(self):
        native = runpy.run_path(str(ROOT / ".github/actions/setup/run.py"))
        calls = []

        def opener(url, timeout):
            calls.append(url)
            return io.BytesIO(PAYLOAD)

        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "archive.zip"
            with self.assertRaises(ValueError):
                native["download"](URL, "0" * 64, destination, opener=opener, sleep=self.fail)
            self.assertFalse(destination.exists())
        self.assertEqual(len(calls), 1)

    def test_download_exhaustion_names_url_and_attempts(self):
        native = runpy.run_path(str(ROOT / ".github/actions/setup/run.py"))
        waits = []

        def opener(url, timeout):
            raise urllib.error.HTTPError(url, 504, "timeout", Message(), None)

        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(RuntimeError) as raised:
                native["download"](URL, "0" * 64, Path(temporary) / "archive.zip", opener=opener, sleep=waits.append)
        attempts = native["DOWNLOAD_ATTEMPTS"]
        self.assertIn(URL, str(raised.exception))
        self.assertIn(f"after {attempts} attempts", str(raised.exception))
        self.assertEqual(len(waits), attempts - 1)


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
