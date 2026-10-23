# Native Build Contract

## Ownership

The existing Helios integration worktree owns this consumer migration. Hub owns shared generators and recipes. Wait for the validated Hub override before rendering; use stkr project update --no-provider with provider credentials disabled.

## Execution

One Build / Native job runs per selected platform/profile. Its product producer runs first; its tests producer runs only when selected consumers require test binaries. Clippy immediately precedes each compilation with matching packages, features, target and profile. Keep product and tests artifacts separate and preserve firmware-target proof.

Mise tasks own exact feature selection. Do not duplicate feature lists in stkr.toml or make installer:dev compile tests. Test and Package consume Build artifacts independently; required tests still gate integration and publication.

The Python wheel's matched Clippy reads Maturin's feature declaration from the package pyproject.toml, preserving its extension-module configuration without a duplicate feature list.

The optional Tests producer retains all eight existing all-features/no-default-features selectors for the workspace, facade, node library and embedded binaries as Clippy compatibility proofs. Workspace/facade all-targets selectors retain example and bench lint coverage. Clippy supersedes Cargo check for these selectors; no duplicate Cargo check remains. The selected test profile is dev or release-tests. Shipping CLI, wheel and Linux firmware builds run matched Clippy; test archive selectors remain `--workspace --tests --exclude helios-mesh-python --features helios-mesh/testing`. Compiler warning denial stays in child environments.

## Routing And Removal

Keep platform-specific ci.jobs entries for id = "build". Remove obsolete build:rust:tests job entries and compilation routing from noncompiling check:rust. Preserve explicit compilation needs such as coverage. Review release overrides against their actual compilation responsibility rather than deleting them blindly.

## Acceptance

Generated native jobs have valid expressions even with no selected tests. Product and test behavior, embedded target checks, packaging and required native gates remain intact. Retire superseded generated files and stale documentation. Reconcile required check names only when the validated candidate is pushed, preserving every other protection.
