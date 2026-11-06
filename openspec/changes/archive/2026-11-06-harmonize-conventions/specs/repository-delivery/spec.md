## ADDED Requirements

### Requirement: Project-Owned Automation

The repository SHALL own its native CI graph and publication tooling, separate local authoring policy from runtime configuration and preserve Linux and Windows requirements.

#### Scenario: Development And Review Channels

- **WHEN** a dev push or pull request runs
- **THEN** dev executes quick checks and Product Build only, and reviews additionally execute deterministic tests and publication rehearsal where applicable

#### Scenario: Mutable Nightly

- **WHEN** changed dev source is selected for nightly
- **THEN** release-profile build and package gate publication, suites run independently, started publication survives supersession and final notes record suite results

### Requirement: Actionable Connection Failures

Host connection diagnostics SHALL name the failing stage and destination while preserving typed errors, operation deadlines and protocol 14.

#### Scenario: Failed Connection Or GPIO Request

- **WHEN** a gateway, device selection or AUX GPIO path fails
- **THEN** deterministic proof covers any changed behavior and no unsupported hardware cause is asserted
