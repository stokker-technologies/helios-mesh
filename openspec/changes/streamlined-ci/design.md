# Migration Design

## Inputs
The authoritative upstream contracts are in the Hub fixes worktree at openspec/changes/machine-work-root/specs/ci-contract/spec.md and specs/ci-cache-impact/spec.md. Parent owns those decisions; product owner supplies implementation details and raises contract changes.

## Ownership
Use this product's chore/migrate-hub-ci branch and migrate-hub-ci worktree. The product agent owns authored product changes and canonical regeneration; parent owns Hub integration and final cross-product coordination. No direct dev commits.

## Sequence
Record baseline failures before edits. Migrate authored sources against the stable Hub interface, then regenerate only with the rebuilt Hub handed off by parent. Run targeted tests while implementing; once source closes run complete applicable available-local proof, repair failures, then publish and monitor every required provider platform through merge.

## Baseline
Windows is the current local platform. Ordinary candidate checks/builds/tests/package and required dry-release proof are baseline scope; nightly coverage is not implicitly included. No benchmark run requested.

## Portability
No simulator; CI provides unavailable native platform proof. Fix baseline failures without weakening checks. Maintain public CI without private Hub dependencies or local-only paths. Preserve machine environment and mandatory hooks.

## Source Inventory
The product owner edits mise.toml, stkr.toml, .agents/GUIDANCE.md, README.md and tools/python/src/helios_mesh_tools/build/product.py. Canonical generation owns provider workflows, managed Python tooling, generated agent guidance and managed tool locks. Preserve Rust/Python product APIs, wire version 14, firmware exports and release ownership.

The retired blanket Linux libudev-dev/pkg-config installation has no owner in Cargo.lock and is removed. Task renaming preserves each behavioral lane. Built-wheel stub verification becomes check:python:generated. Local proof runs ordinary producers and consumers sequentially without a ci aggregate; nightly coverage remains explicit. No new cache or impact customization is needed for this migration.

Repository-owned .gitattributes fixes TOML and LICENSE to LF because Taplo's default output and the canonical Apache license bytes must survive Windows checkouts with core.autocrlf enabled. Preserve the machine Git configuration and license text; format authored files, recreate LICENSE through the selected canonical skeleton, and regenerate managed files through their owner.

## Specification Lifecycle
After complete source and full applicable local proof, finish folds the specification into the candidate before publication. The parent's native goal retains publication, monitoring, repair, protected merge and merged-revision verification. Workflow Follow-Up carries those obligations without marking unperformed delivery complete or creating a second archival PR.
