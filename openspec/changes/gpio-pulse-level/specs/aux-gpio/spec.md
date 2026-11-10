# Auxiliary GPIO Spec Delta (Proposed)

## ADDED Requirements

### Requirement: Pulse Level

A GPIO pulse request SHALL name the pulse level, and the node SHALL drive the pin at the opposite level before and after the pulse.

#### Scenario: High Pulse

- **WHEN** a client requests a high pulse of a duration on an auxiliary pin
- **THEN** the node drives the pin low, drives it high for that duration and drives it low again

#### Scenario: Low Pulse

- **WHEN** a client requests a low pulse of a duration on an auxiliary pin
- **THEN** the node drives the pin high, drives it low for that duration and drives it high again
