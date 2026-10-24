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

Provider Product and Tests Build jobs SHALL run independently in parallel with their own canonical feature sets. Tests SHALL NOT require Product outputs. Downstream lanes SHALL receive their required artifacts.

#### Scenario: Producer Timing

- **WHEN** the same candidate is validated on a native platform and profile
- **THEN** Tests Build duration is no greater than Product Build duration and the warm full pipeline finishes below 45 minutes, targeting 30
- **AND** a regression requires reassessment without silently reducing coverage or changing architecture or shipping feature scope

### Requirement: Built-In Cargo Profiles

Projects SHALL declare no workspace, package or build-dependency compiler-profile overrides. Product SHALL use built-in dev or release, Bench built-in bench, and Tests and their preceding Clippy built-in unoptimized test in either graph context. Artifact and report namespaces SHALL retain the requested graph profile. Infrastructure cache policy may disable incremental compilation.

#### Scenario: Release Graph Tests

- **WHEN** a release graph requests Product and Tests
- **THEN** Product selects default release while Tests and matching Clippy select default test
- **AND** both producers retain release artifact namespaces and existing features, coverage and publication gates

### Requirement: Canonical Landing Page And Declared Prose Impact

The root README and CONTRIBUTING SHALL follow the shared product-readmes structure. Existing executable examples SHALL remain unchanged during the prose/layout migration. The landing page SHALL use an approved existing logo without fabricating media or registry publication status.

#### Scenario: Documentation Candidate

- **WHEN** only root README, CONTRIBUTING or explicitly audited ordinary documentation prose changes
- **THEN** declared impact rules select repository structure/configuration checks without application compilation or test lanes
- **AND** mixed and unknown changes retain the union or conservative full graph respectively
- **AND** LICENSE changes retain Rust packaging and its Product prerequisites because the firmware ZIP embeds that file
### Requirement: Correctness Hang Guards And Complete Artifact Streams

Every correctness-test hang guard SHALL be at least 30 seconds, including explicit overrides, while larger declared budgets SHALL be preserved. Historical durations SHALL NOT tighten correctness deadlines. Responsiveness below 10 seconds SHALL be assessed separately from the liveness guard and by execution phase.

#### Scenario: Artifact Ends Before Decompressor Output

- **WHEN** a valid tar artifact contains trailing padding beyond its end-of-archive marker
- **THEN** extraction drains the decompressor output before closing the pipe and requires its successful exit
- **AND** a corrupt compressed stream still fails rather than ignoring its nonzero status

### Requirement: Serial Local Simulation

Each local CI simulation SHALL execute one declared job at a time without imposing thread, CPU or memory limits. Independent repository gates MAY run concurrently. Declared platform coverage, artifact dependencies and failure propagation SHALL remain unchanged.

#### Scenario: Local Graph Execution

- **WHEN** local simulation validates a candidate
- **THEN** each job finishes before the next begins, failed prerequisites block their consumers, and independent jobs still execute
- **AND** the simulator applies no resource allocation flags, worker overrides or memory controls
