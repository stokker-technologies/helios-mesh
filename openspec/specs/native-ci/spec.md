# Native CI Specification

## Purpose
Produce native shipping and optional test artifacts in one shared job per platform and profile, preserving compiler configuration and required platform proof.

## Requirements

### Requirement: Shared Native Build

The pipeline SHALL have one native Build job per selected platform/profile that produces product artifacts and, when needed, test artifacts using their respective declared feature sets.

#### Scenario: Tests Selected

- **WHEN** selected jobs consume native tests
- **THEN** Build runs matched Clippy and product/test compilation in the same job and exports separate artifacts for parallel Test and Package consumers.

#### Scenario: Installer Only

- **WHEN** a user invokes the development installer task
- **THEN** only required product outputs are built and test binaries are not added by default.

#### Scenario: Native And Embedded Proof

- **WHEN** the migration candidate is validated
- **THEN** existing Linux, Windows and configured embedded-target requirements remain enforced with no weakened checks.
