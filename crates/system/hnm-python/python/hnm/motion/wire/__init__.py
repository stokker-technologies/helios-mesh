# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Protocol-shaped motion types, demoted here as an escape hatch.

`NodeHandle.configure` takes the flat `hnm.motion.StepperConfig` instead of
`Configuration`; these nested wire types remain reachable for advanced or
interop use, not because the primary `hnm.motion` surface needs them.
"""

# === Imports === #

# Standard Library Imports
# Third Party Imports
# Local Imports
from hnm._native import Configuration as Configuration
from hnm._native import Limits as Limits
from hnm._native import Mechanics as Mechanics
from hnm._native import Microsteps as Microsteps
from hnm._native import Power as Power
from hnm._native import Settings as Settings
from hnm._native import StepperConfiguration as StepperConfiguration

# Type Checking Imports

# === Public Exports === #

__all__ = [
    "Configuration",
    "Limits",
    "Mechanics",
    "Microsteps",
    "Power",
    "Settings",
    "StepperConfiguration",
]

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #

# === Private Functions === #

# === Entry Point === #
