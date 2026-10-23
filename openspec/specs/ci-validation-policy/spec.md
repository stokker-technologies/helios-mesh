# CI Validation Policy Specification

## Purpose
Define native cache isolation, supersession cancellation, release protection and ordinary job budgets for Helios Mesh CI.

## Requirements

### Requirement: Adopt Canonical CI Policy
The product SHALL use safe native PR cache writes, cancellation of superseded ordinary validation with release protection, and a 120-minute ordinary job default.

#### Scenario: Repeated Pull Request Commit
- WHEN a new commit supersedes an ordinary PR candidate
- THEN validation can cancel the older work and reuse compatible PR-scoped caches without making them visible to trusted base-branch jobs.

#### Scenario: Release Finalization
- WHEN a release review merges into dev or a stable release tag runs
- THEN newer ordinary validation cannot interrupt its required tagging/publication handoff.

#### Scenario: Routine Job Budget
- WHEN a job has no justified exceptional timeout
- THEN it uses the canonical 120-minute default while test-level deadlines remain unchanged.
