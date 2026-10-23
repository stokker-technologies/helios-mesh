# Compile Runner Routing Specification

## Purpose
Reserve local native runners for crate-compiling jobs while hosted runners execute artifact consumers and other work.

## Requirements

### Requirement: Route By Actual Crate Compilation
CI SHALL reserve local runners for jobs that compile crates and route other jobs to platform-matching hosted runners.

#### Scenario: Compilation Producer
- WHEN a job invokes crate compilation directly or through a nested task
- THEN its configured local native runner remains selected.

#### Scenario: Artifact Consumer
- WHEN a job only executes or packages existing artifacts without crate compilation
- THEN it uses a hosted runner for the required native platform with unchanged bootstrap and gates.
