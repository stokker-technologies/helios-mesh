# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***
"""Testing Policy From `[conventions.testing]` And The Expected Lane Set.

Keys, all optional:

    [conventions.testing]
    deadlines = { unit = 30, integration = 120, acceptance = 900, system = 900 }
    bench_regression_percent = 20

    [[conventions.testing.overrides]]
    category = "acceptance"
    language = "rust"
    select = "test(=cycle::runs)"
    timeout_seconds = 300
    reason = "Streams a full capture"

Deadlines are the per-story maximum of a lane in seconds. An override
replaces the deadline of the stories it selects: for Rust `select` is a nextest filterset, for
the other languages a glob over `<file>::<story>`. Acceptance and system lanes receive the whole
Product and Tests builds of their profile and target, so lanes declare no inputs.

The expected lanes are the `test:<language>:<category>` leaf tasks of `mise.toml`.
"""

# === Imports === #

# Standard Library Imports
import fnmatch
import math
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# Third Party Imports
# Local Imports
from ..policy import ignored

# Type Checking Imports

# === Public Exports === #

# === Constants === #

CATEGORIES = ("unit", "integration", "acceptance", "system")
BENCH_CATEGORIES = ("unit", "integration", "system")
LANGUAGES = ("rust", "python", "typescript", "javascript")
PROFILES = ("dev", "release")
DEADLINES = {"unit": 30, "integration": 120, "acceptance": 900, "system": 900}

# Minimum Correctness-Test Hang Guard; Performance Belongs To Benchmarks.
MINIMUM_TIMEOUT = 30
COVERAGE_FACTOR = 5
OVERRIDE_KEYS = {"category", "language", "select", "timeout_seconds", "reason"}

# === Types === #

# === Classes === #


class TestError(Exception):
    """A preflight, policy or runner failure reported as one clear message."""


@dataclass(frozen=True)
class Override:
    category: str
    language: str
    select: str
    timeout_seconds: int
    reason: str


@dataclass(frozen=True)
class Policy:
    deadlines: dict[str, int]
    overrides: tuple[Override, ...]
    bench_regression_percent: float = 20

    def deadline(self, category: str, coverage: bool = False) -> int:
        """The lane maximum per story, multiplied for instrumented runs."""
        return self.deadlines[category] * (COVERAGE_FACTOR if coverage else 1)

    def selected(self, category: str, language: str) -> list[Override]:
        return [entry for entry in self.overrides if entry.category == category and entry.language == language]

    def matched(self, category: str, language: str, story: str) -> Override | None:
        """The first glob override of a script or Python story."""
        for entry in self.selected(category, language):
            if fnmatch.fnmatchcase(story, entry.select):
                return entry
        return None


# === Public Functions === #


def root(start: Path | None = None) -> Path:
    """The project root: the nearest directory holding project policy."""
    current = (start or Path.cwd()).resolve()
    for candidate in (current, *current.parents):
        if (candidate / "tools/project.toml").is_file():
            return candidate
    raise TestError("no project policy file found above the working directory")


def load(directory: Path) -> Policy:
    document = _toml(directory / "tools/project.toml")
    testing = document.get("conventions", {}).get("testing", {})
    if not isinstance(testing, dict):
        raise TestError("[conventions.testing] must be a table")

    for key in testing.keys() - {"deadlines", "bench_regression_percent", "overrides"}:
        ignored(directory, f"conventions.testing.{key}")
    deadlines = _numbers(directory, testing.get("deadlines", {}), "deadlines", DEADLINES, MINIMUM_TIMEOUT)

    overrides = []
    for entry in testing.get("overrides", []):
        if not isinstance(entry, dict) or not OVERRIDE_KEYS <= set(entry):
            raise TestError("each override declares " + ", ".join(sorted(OVERRIDE_KEYS)))
        if entry["category"] not in CATEGORIES or entry["language"] not in LANGUAGES:
            raise TestError(f"override names an unknown lane: {entry}")
        if not _positive(entry["timeout_seconds"], MINIMUM_TIMEOUT) or not str(entry["reason"]).strip():
            raise TestError(f"override needs a timeout of at least 30 seconds and a reason: {entry}")
        for key in entry.keys() - OVERRIDE_KEYS:
            ignored(directory, f"conventions.testing.overrides.{len(overrides)}.{key}")
        overrides.append(Override(**{key: entry[key] for key in OVERRIDE_KEYS}))
    threshold = testing.get("bench_regression_percent", 20)
    if isinstance(threshold, bool) or not isinstance(threshold, (int, float)) or not math.isfinite(threshold) or threshold <= 0:
        raise TestError("bench_regression_percent must be finite and positive")
    return Policy(
        deadlines,
        tuple(overrides),
        float(threshold),
    )


def lanes(directory: Path) -> list[tuple[str, str]]:
    """The expected `(category, language)` lanes: the `test:*:*` leaves in `mise.toml`."""
    path = directory / "mise.toml"
    tasks = _toml(path).get("tasks", {}) if path.is_file() else {}
    expected = []
    for name in tasks:
        parts = name.split(":")
        if len(parts) == 3 and parts[0] == "test" and parts[2] in CATEGORIES and parts[1] in LANGUAGES:
            expected.append((parts[2], parts[1]))
    return sorted(expected, key=lambda lane: (CATEGORIES.index(lane[0]), lane[1]))


# === Private Functions === #


def _toml(path: Path) -> dict[str, Any]:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise TestError(f"read {path.name}: {error}") from error


def _numbers(directory: Path, value: object, name: str, defaults: dict[str, int], minimum: int) -> dict[str, int]:
    if not isinstance(value, dict):
        raise TestError(f"conventions.testing.{name} keys are {', '.join(defaults)}")
    for key in value.keys() - defaults.keys():
        ignored(directory, f"conventions.testing.{name}.{key}")
    value = {key: number for key, number in value.items() if key in defaults}
    if not all(_positive(number, minimum) for number in value.values()):
        raise TestError(f"conventions.testing.{name} values must be integers >= {minimum}")
    return {**defaults, **value}


def _positive(value: object, minimum: int = 1) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= minimum


# === Entry Point === #
