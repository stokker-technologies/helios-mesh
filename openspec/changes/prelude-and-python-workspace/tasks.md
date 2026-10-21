# Tasks

## 1. Prelude

- [x] 1.1 Add `helios_mesh::prelude` with feature-gated re-exports and migrate README, crate README examples and transport tests to it; keep existing paths. Proof: feature-gated identity tests compile in `check:rust`, including facade-only all-feature and no-default-feature checks.

## 2. Python Workspace

- [x] 2.1 Move the uv workspace to the repository root (`tools/python` member, distribution excluded), relocate the lock, remove the `UV_PROJECT_ENVIRONMENT` redirect from `mise.toml`, and update rendered cache keys. Proof: `mise run check` and the 41-test Python acceptance lane pass from this sibling worktree using its root environment; declared Python unit/integration lanes are absent.
- [x] 2.2 Reconcile the rendered `pytest.py` with the Hub template through `stkr project update` (hand edits are drift); raise the Python line length to 140 where the Hub allows it. Proof: restored HEAD then regenerated without conflicts; the template uses `sys.executable`, strips the redirect, and `check:python` passes at width 140.

## 3. Validation

- [x] 3.1 Full Linux lanes and the Windows VM graph green on the combined tree; README claims verified. Proof: Linux check, development Product/Tests builds, Rust unit 215/215, integration 56/56, acceptance 3/3, Python acceptance 41/41, and generated bindings for 7 modules passed. The stkr 0.1.7 Windows PR graph passed both builds, Rust check, unit 215/215, integration 56/56, acceptance 3/3 and Rust packaging; 31 jobs passed, none failed, 52 were event-gated and 13 Linux jobs were unselected. README imports and root-workspace claims match these verified paths.
