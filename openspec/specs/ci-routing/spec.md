# CI Routing Specification

## Purpose
Define GitHub-hosted runner routing while preserving native tasks, required gates and platform coverage.

## Requirements

### Requirement: Hosted Runner Selection

Every GitHub CI job SHALL use the platform-matching GitHub-hosted runner default without local runner overrides or provider runner variables.

#### Scenario: Native Work

- **WHEN** a Linux or Windows job is rendered
- **THEN** it uses the corresponding GitHub-hosted runner with unchanged tasks, bootstrap and required gates
