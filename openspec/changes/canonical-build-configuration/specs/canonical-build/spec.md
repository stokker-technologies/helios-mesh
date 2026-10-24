# Canonical Build

## ADDED Requirements

### Requirement: Canonical CI Configuration

Provider CI SHALL build and test canonical shipping configurations only, with matching Clippy and compatible Cargo reuse. Required native platforms and actual shipped firmware targets SHALL retain their proof. Shipping features SHALL NOT expand merely to match broader tests.

#### Scenario: Ordinary Build

- **WHEN** CI requests Product and Tests
- **THEN** matching canonical outputs are produced and exported without the noncanonical compatibility matrix

### Requirement: Explicit Extended Validation

Noncanonical feature combinations and instrumented tests SHALL remain available through explicit local extended tasks and SHALL NOT run automatically in provider CI or ordinary validation.

#### Scenario: Extended Local Run

- **WHEN** the developer explicitly requests extended validation
- **THEN** its required builds and preserved extended checks execute with documented extra cost


### Requirement: Independent Parallel Producers

Product and Tests Build jobs SHALL run independently in parallel with their own canonical feature sets. Tests SHALL NOT require Product outputs. Downstream lanes SHALL receive their required artifacts.

#### Scenario: Producer Timing

- **WHEN** the same candidate is validated on a native platform and profile
- **THEN** Tests Build duration is no greater than Product Build duration and the warm full pipeline finishes below 45 minutes, targeting 30
- **AND** a regression requires reassessment without silently reducing coverage or changing architecture or shipping feature scope
