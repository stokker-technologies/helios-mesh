# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***


"""Prove Concurrent Native Task Entry And Exit Status Propagation"""

# === Imports === #

# Standard Library Imports
import os
import queue
import subprocess
import sys
import threading
import unittest

# Third Party Imports

# Local Imports

# Type Checking Imports

# === Public Exports === #

# === Constants === #

CHILD = """
import sys
print("Entered", flush=True)
sys.stdin.readline()
sys.exit(23)
"""

# === Types === #

# === Classes === #


class NativeTaskTests(unittest.TestCase):
    def test_children_enter_concurrently_and_preserve_exit_status(self):
        environment = dict(os.environ, RUSTC_WRAPPER="")
        command = [sys.executable, "-m", "helios_mesh_tools.build.cache", "--", "python", "-u", "-c", CHILD]
        processes = []
        try:
            for _ in range(2):
                process = subprocess.Popen(
                    command, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
                )
                processes.append(process)
                self.read_until(process, "Entered")
            for process in processes:
                self.assertIsNone(process.poll())
                assert process.stdin is not None
                process.stdin.write("release\n")
                process.stdin.flush()
            for process in processes:
                self.assertEqual(process.wait(timeout=60), 23)
        finally:
            for process in processes:
                if process.poll() is None:
                    process.kill()
                process.communicate(timeout=60)

    def read_until(self, process, marker):
        result = queue.Queue()

        def read():
            while line := process.stdout.readline():
                if marker in line:
                    result.put(line)
                    return
            result.put(None)

        threading.Thread(target=read, daemon=True).start()
        line = result.get(timeout=60)
        self.assertIsNotNone(line, f"holder exited before {marker}: {process.poll()}")
        return line


# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
