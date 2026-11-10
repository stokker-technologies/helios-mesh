# GPIO Pulse Level

Status: proposed on 2026-11-10; not locked. Implementation waits for the two
open decisions in `design.md`.

## Why

`gpio-pulse` has a fixed polarity that the board decides: the Sidecar auxiliary
pins pulse active low and then release to high impedance. A caller that needs
a high pulse on a line that rests low cannot ask for it, and the command's own
description says "high pulse" while the board emits a low one.

## What Changes

- The pulse request carries the pulse level. A high pulse drives the pin low,
  drives it high for the duration and drives it low again; a low pulse is the
  mirror image. The pin is push-pull at 12 mA throughout.
- CLI: `gpio-pulse <id> high|low <ms>`.
- Rust: `sidecar.pulse_gpio(AuxGpioId(1), AuxGpioLevel::High, duration)`.
- Python: `pulse_gpio(AuxGpioId(1), AuxGpioLevel.HIGH, timedelta(...))`.
- The fixed per-board pulse polarity is deleted.

## Impact

- Breaking wire change to `PulseAuxGpio`
  (`crates/common/helios-mesh-protocol/src/wire/node.rs`): boards need
  reflashing, and clients and firmware move together.
- Touches the firmware pulse handling
  (`crates/firmware/helios-mesh-core/src/mesh/coordinator/`,
  `crates/firmware/helios-mesh-nodes/src/bin/sidecar.rs`), the Rust client
  (`crates/system/helios-mesh/src/nodes/mod.rs`), the Python binding and
  stubs, the CLI (`crates/system/helios-mesh-cli/src/commands/gpio_pulse.rs`),
  the README and their tests.
