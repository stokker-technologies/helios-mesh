# Tasks

## 1. Prelude

- [x] 1.1 Add `helios_mesh::prelude` with feature-gated re-exports and migrate README, crate README examples and transport tests to it; keep existing paths. Proof: feature-gated identity tests compile in `check:rust`, including facade-only all-feature and no-default-feature checks.

## 2. Python Workspace

- [x] 2.1 Move the uv workspace to the repository root (`tools/python` member, distribution excluded), relocate the lock, remove the `UV_PROJECT_ENVIRONMENT` redirect from `mise.toml`, and update rendered cache keys. Proof: `mise run check` and the 41-test Python acceptance lane pass from this sibling worktree using its root environment; declared Python unit/integration lanes are absent.
- [x] 2.2 Reconcile the rendered `pytest.py` with the Hub template through `stkr project update` (hand edits are drift); raise the Python line length to 140 where the Hub allows it. Proof: restored HEAD then regenerated without conflicts; the template uses `sys.executable`, strips the redirect, and `check:python` passes at width 140.

## 3. Validation

- [ ] 3.1 Full Linux lanes and the Windows VM graph green on the combined tree; README claims verified. Linux checks, both development builds, all four declared lanes and generated bindings pass. Windows cannot reach execution: Hub `crates/stkr/src/ci/simulate/mod.rs:246` skips the `validated` job without publishing empty declared outputs, and `expression.rs:287` rejects `needs.validated.outputs.validated`. Hub must supply empty outputs for skipped jobs (preserving errors for unsupported contexts), cover the rendered PR gate, and reinstall the simulator before this task can pass.
