# Design

## Context

The uncommitted work of 2026-10-21 (prelude module, root `pyproject.toml` workspace, `tools/python` lock relocation, pytest tooling using `sys.executable`, `stkr.toml` barrel exclusion) is adopted as one change validated as a whole.

## Decisions

- The prelude is a plain re-export module (`pub use crate::{…}`) with feature-gated sections; it is the only barrel the convention check tolerates, declared in `stkr.toml`.
- `tools/python` keeps no own lock; the root lock is authoritative. The distribution crate keeps its own manifest and lock.
- The rendered `pytest.py` must come from the Hub template through `stkr project update`, never hand-edited; if the template lacks the `sys.executable` behaviour, the change goes to the Hub first.

## Source Inventory And Acceptance

This worktree and `feat/feature-drop-2026-10-21` own the combined change, with one writer.
The prelude, facade transport tests and both README examples form the Rust slice;
feature-gated identity tests compile with the existing all-feature and no-default-feature checks.
There are no standalone Rust examples in the facade. Existing protocol-specific imports remain.
The root workspace, lock relocation, mise environment, rendered workflow cache keys and
Python runner form the Python slice. Managed refresh also adopts current release templates.
OpenSpec requires the Hub-pinned Node and OpenSpec tools and their mise lock entries.
The Python distribution remains excluded and independently locked. No wire or version changes apply.
Source editing precedes Linux check, Product/Tests builds and declared test lanes, followed by
the Windows PR simulation with stkr 0.1.6. Publication waits for both platforms to pass;
the authorized destination is one draft PR into `dev`.

The Windows simulator requires stkr 0.1.6's single-label runner schema;
existing Windows and nightly label values are preserved as strings.
Its generated release archive runner uses the required `release-tests` profile,
inheriting release optimization/debug settings with LTO and incremental compilation
disabled and 16 codegen units. Development build and test commands are unchanged.
The upgraded Rust gate includes pinned cargo-machete. Its finding removes the unused
direct embassy-sync dependency from the nodes crate; shared consumers retain theirs.
