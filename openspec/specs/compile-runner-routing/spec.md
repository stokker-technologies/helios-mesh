# Compile Runner Routing Specification

## Purpose
Run compilation producers and artifact consumers on GitHub-hosted runners.

## Requirements

### Requirement: Hosted Compilation And Consumption

CI SHALL use platform-matching GitHub-hosted runners for every job, including jobs that compile crates.

#### Scenario: Compilation Producer

- **WHEN** a job invokes crate compilation directly or through a nested task
- **THEN** it uses the required platform's GitHub-hosted runner with unchanged compiler selectors and gates

#### Scenario: Artifact Consumer

- **WHEN** a job executes or packages existing artifacts without crate compilation
- **THEN** it uses the required platform's GitHub-hosted runner with unchanged bootstrap and gates
