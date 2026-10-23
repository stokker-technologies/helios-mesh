# CI Migration Specification

## Purpose
Define native CI task taxonomy, artifact ownership, required platform gates and verified delivery for Helios Mesh.

## Requirements

### Requirement: Adopt The Agreed Hub Contract
The product SHALL migrate its authored tasks and configuration and canonically generated tooling/workflows to the locked Hub CI contracts while preserving required native platform coverage, artifact dependencies and release behavior.

#### Scenario: Candidate Delivery
- **WHEN** complete source passes applicable local proof
- **THEN** publish the candidate PR to dev, monitor and repair required provider checks until green, merge with protections enforced and verify the merged revision.

### Requirement: Honest Baseline And Repairs
The initial full baseline SHALL run before implementation as explicitly requested; every observed existing failure remains actionable and cannot waive final gates. Only targeted proof runs during migration until source closes. Optional nightly workloads are excluded unless explicitly requested.

#### Scenario: Baseline Failure
- **WHEN** an initial lane fails or a prerequisite prevents it running
- **THEN** record the actual result, repair the cause within the authorized migration and prove required behavior before delivery.

### Requirement: Firmware Tool Setup Follows Its Consumer

CI MUST install firmware conversion tooling only for entrypoints that need it, with identical pinned behavior on hosted and local runners.

#### Scenario: Lightweight Job

- **WHEN** a check or test job does not invoke firmware conversion and has no other native dependency on its bootstrap packages
- **THEN** it does not install the converter or run that bootstrap script

#### Scenario: Cold Firmware Packaging

- **WHEN** Linux Rust packaging runs with no converter already installed
- **THEN** native mise resolution supplies the pinned converter and required dependencies and produces the same declared firmware assets

#### Scenario: Release Caller

- **WHEN** a release or aggregate task invokes firmware packaging
- **THEN** the same declared tool selection applies without reliance on a prior job or developer installation

#### Scenario: Runner Metadata

- **WHEN** Hub renders GitHub CI with literal custom runner labels
- **THEN** actionlint accepts the declared labels through generated native configuration without diagnostic suppression
