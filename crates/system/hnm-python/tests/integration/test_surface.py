# ***
#
# Helios Node Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Parity floor: runtime surface, stub exports, and `__all__` stay aligned."""

# === Imports === #

# Standard Library Imports
import ast
from importlib import import_module
from pathlib import Path
from types import ModuleType

# Third Party Imports
import pytest

# Local Imports
import hnm
import hnm.gateway
import hnm.journal
import hnm.motion
import hnm.motion.wire
import hnm.node

# Type Checking Imports

# === Public Exports === #

# === Constants === #

MODULES = [hnm, hnm.gateway, hnm.journal, hnm.motion, hnm.motion.wire, hnm.node]

# === Types === #

# === Classes === #

# === Public Functions === #


def module_all(module: ModuleType) -> list[str]:
    names: list[str] = getattr(module, "__all__")  # noqa: B009
    return names


def stub_path(module: ModuleType) -> Path:
    runtime_path = module.__file__
    assert runtime_path is not None
    return Path(runtime_path).with_suffix(".pyi")


def exported_stub_names(path: Path) -> set[str]:
    """Names a stub file exports: definitions plus redundant `as` aliases."""
    tree = ast.parse(path.read_text(encoding="utf-8"))
    names: set[str] = set()
    for statement in tree.body:
        if isinstance(statement, ast.ClassDef | ast.FunctionDef | ast.AsyncFunctionDef):
            names.add(statement.name)
        elif isinstance(statement, ast.AnnAssign) and isinstance(statement.target, ast.Name):
            names.add(statement.target.id)
        elif isinstance(statement, ast.Assign):
            for target in statement.targets:
                if isinstance(target, ast.Name):
                    names.add(target.id)
        elif isinstance(statement, ast.ImportFrom):
            for alias in statement.names:
                if alias.asname == alias.name:
                    names.add(alias.name)
    return {name for name in names if not name.startswith("__")}


@pytest.mark.parametrize("module", MODULES, ids=lambda module: str(module.__name__))
def test_runtime_all_exports(module: ModuleType) -> None:
    assert set(module_all(module)) == exported_stub_names(stub_path(module))


@pytest.mark.parametrize("module", MODULES, ids=lambda module: str(module.__name__))
def test_name_all_runtime(module: ModuleType) -> None:
    missing = [name for name in module_all(module) if name not in dir(module)]
    assert missing == []


def test_native_module_export() -> None:
    """Every public class resolves to the same object as its `_native` twin."""
    native = import_module("hnm._native")
    for module in MODULES:
        for name in module_all(module):
            member: object = getattr(module, name)
            if isinstance(member, ModuleType):
                continue
            assert getattr(native, name) is member


# === Private Functions === #

# === Entry Point === #
