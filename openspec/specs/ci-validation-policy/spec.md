# CI Validation Policy Specification

## Purpose
Define native cache isolation, supersession cancellation, release protection and ordinary job budgets for Helios Mesh CI.

## Requirements

### Requirement: Adopt Canonical CI Policy
The product SHALL restore compatible caches without PR writes, cancel superseded ordinary validation while protecting started publication, and use a 120-minute ordinary job default.

#### Scenario: Repeated Pull Request Commit
- WHEN a new commit supersedes an ordinary PR candidate
- THEN validation can cancel the older work and restore compatible profile-scoped caches without saving them.

#### Scenario: Release Finalization
- WHEN a release review merges into dev or a stable release tag runs
- THEN newer ordinary validation cannot interrupt its required tagging/publication handoff.

#### Scenario: Routine Job Budget
- WHEN a job has no justified exceptional timeout
- THEN it uses the canonical 120-minute default and doubled hosted-runner correctness-test hang guards while operation deadlines remain unchanged.
