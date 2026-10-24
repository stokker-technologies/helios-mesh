# Native CI Specification

## Purpose
Produce native shipping and optional test artifacts in independent parallel jobs per platform and profile, preserving canonical compiler configuration and required platform proof.

## Requirements

### Requirement: Independent Native Builds

The pipeline SHALL have independent Product and Tests Build jobs per selected platform/profile, running in parallel with their respective canonical feature sets. Tests Build SHALL NOT require Product output.

#### Scenario: Tests Selected

- **WHEN** selected jobs consume native tests
- **THEN** the independent producers run matched Clippy and compilation and export separate artifacts; Test receives its required Product and Tests artifacts while Package depends only on Product.

#### Scenario: Installer Only

- **WHEN** a user invokes the development installer task
- **THEN** only required product outputs are built and test binaries are not added by default.

#### Scenario: Native And Embedded Proof

- **WHEN** the migration candidate is validated
- **THEN** existing Linux, Windows and configured embedded-target requirements remain enforced with no weakened checks.
