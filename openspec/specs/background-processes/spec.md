# Background Processes Specification

## Purpose
Keep noninteractive validation silent on Windows while preserving process streams, status, cancellation and portable fixture validation.

## Requirements

### Requirement: Invisible Background Validation

Noninteractive validation MUST avoid visible Windows terminal windows while preserving output, exit status, cancellation, deadlines and native console signaling when needed. Interactive and PTY behavior MUST remain functional.

#### Scenario: Background Test Execution

- **WHEN** native Windows validation starts child commands
- **THEN** no visible terminal opens and process output and exit status remain available

#### Scenario: Other Platforms

- **WHEN** validation runs on a hosted or local non-Windows runner
- **THEN** native behavior and required gates are unchanged

### Requirement: Portable Fixture Validation

Fixture stamp keys and scoped required/root selectors MUST use consistent portable paths so Windows validation rejects stale built inputs with the same behavior as other platforms.

#### Scenario: Scoped Windows Fixture Changes

- **WHEN** a stamped Windows fixture changes and validation selects its portable root
- **THEN** validation reports the stale fixture instead of skipping it because path separators differ
