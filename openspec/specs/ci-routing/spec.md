# CI Routing Specification

## Purpose
Define project-owned runner routing while preserving native tasks, required gates and hosted defaults.

## Requirements

### Requirement: Declarative Runner Selection

CI SHALL select runner labels from the project and SHALL use identical tasks and required gates on hosted and local runners.

#### Scenario: Heavy Native Work

- **WHEN** a platform-specific Rust build or test job is rendered with a local override
- **THEN** it uses that platform's declared local runner label

#### Scenario: Lightweight Hosted Check

- **WHEN** a lightweight check has no local runner override
- **THEN** it uses the hosted project default without consulting provider runner variables
