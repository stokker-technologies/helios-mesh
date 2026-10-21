# Design

## Context

The uncommitted work of 2026-10-21 (prelude module, root `pyproject.toml` workspace, `tools/python` lock relocation, pytest tooling using `sys.executable`, `stkr.toml` barrel exclusion) is adopted as one change validated as a whole.

## Decisions

- The prelude is a plain re-export module (`pub use crate::{…}`) with feature-gated sections; it is the only barrel the convention check tolerates, declared in `stkr.toml`.
- `tools/python` keeps no own lock; the root lock is authoritative. The distribution crate keeps its own manifest and lock.
- The rendered `pytest.py` must come from the Hub template through `stkr project update`, never hand-edited; if the template lacks the `sys.executable` behaviour, the change goes to the Hub first.
