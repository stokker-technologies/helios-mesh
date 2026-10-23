# Helios Mesh

**Control embedded nodes over USB and LoRa from Rust or Python.**

Helios Mesh connects an engineering workstation to RP2040 boards that run
motion and GPIO jobs. Connect a node directly over USB, or use a Gateway to
reach Rotator and Sidecar nodes over LoRa. The firmware owns timing, queued
work, cancellation, and cleanup; your application uses typed handles.

[Quick Start](#quick-start) · [Examples](#examples) ·
[Firmware](#firmware) · [Documentation](#documentation)

With a Gateway attached over USB and a Sidecar in its radio network, list
the nodes, inspect one, then schedule a one-second pulse on GPIO 1:

```sh
mise run dev:cli -- nodes
mise run dev:cli -- --node <sidecar-id> status
mise run dev:cli -- --node <sidecar-id> gpio-pulse 1 1000
```

The Sidecar times the pulse locally and drives the pin low during cleanup.
Once admitted, the job continues if the host disconnects. GPIO 1 is D11 on
the built-in Sidecar image; check the
[hardware and GPIO limits](crates/firmware/helios-mesh-board-feather-rp2040/README.md#hardware-and-gpio-safety)
before connecting a load.

## Features

- **USB And LoRa Control**

  Use the same node operations through a direct USB connection or a Gateway.
  Select devices by stable six-digit IDs derived from onboard flash identity.

- **Typed Host APIs**

  Discover Gateway, Rotator, and Sidecar nodes and connect to their concrete
  Rust or Python handles. Python bindings include type stubs and asynchronous
  operation handles.

- **Firmware-Owned Jobs**

  Submit bounded work to independent channels, await completion, or request
  cancellation. Execution and cleanup failures remain distinct in operation
  results; host disconnection does not cancel admitted work.

- **Motion And GPIO**

  Configure two Rotator stepper channels with speed, acceleration, and duty
  limits. Sidecar provides two independent GPIOs with high-impedance, high,
  low, and firmware-timed pulse operations. Motion is open loop and requires
  homing after each restart.

- **Diagnostics**

  Inspect node status and request counters, read the bounded boot-session
  journal, or record its events to JSONL. Local and remote node diagnostics
  use the same CLI command tree.

- **Firmware Authoring**

  Declare node types, typed routes, resources, jobs, and attached hardware in
  Rust. The board validates pin reservations before starting shared buses.
  See the [firmware guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md#authoring-a-node)
  for a complete Sidecar implementation.

## Quick Start

Helios Mesh is pre-1.0. Linux and Windows have native CI coverage; Linux also
builds the RP2040 firmware. Start from source with the pinned toolchain and
[mise](https://mise.jdx.dev).

Linux Rust packaging needs `pkg-config` and the libudev development headers.
On Debian or Ubuntu, install them with
`sudo apt-get install pkg-config libudev-dev`. CI installs these prerequisites
only for Rust packaging through `tools/ci/bootstrap-linux.sh`. The
`package:rust` task selects the pinned firmware converter on Linux, including
when called through `package` or a local release rehearsal.

```sh
git clone https://github.com/stokker-technologies/helios-mesh.git
cd helios-mesh
mise trust
mise run prepare
mise run build --profile dev
mise run dev:cli -- --help
```

The help command needs no hardware. To inspect an already-flashed node,
connect it over USB and run:

```sh
mise run dev:cli -- status
mise run dev:cli -- describe
```

With one attached node, selection is automatic. With multiple nodes, the
single Gateway takes precedence; otherwise use `--device <six-digit-id>` or
`--port <serial-port>`. Add `--node <six-digit-id>` to address a remote node
through a Gateway. See [device selection](crates/system/helios-mesh/README.md#device-selection)
for examples and Linux serial permissions.

For a blank board, follow [Firmware](#firmware). Hardware must match the image's
declared attachments, and RFM95 radios need suitable antennas before power-up.
The [board guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md)
owns wiring and electrical precautions.

## Examples

### Await A Sidecar Pulse In Python

Install the local extension after the build:

```sh
mise run dev:python
```

Run this example with a directly attached Sidecar. It discovers that node,
starts a 250 ms pulse, and waits until execution and cleanup have finished:

```python
import asyncio
from datetime import timedelta

import helios_mesh


async def main():
    devices = await helios_mesh.discover(helios_mesh.NodeType.SIDECAR)
    if not devices:
        raise RuntimeError("Connect a Sidecar over USB")
    sidecar = await devices[0].connect()
    assert isinstance(sidecar, helios_mesh.SidecarNode)

    operation = await sidecar.pulse_gpio(
        helios_mesh.AuxGpioId(1), timedelta(milliseconds=250)
    )
    result = await operation.completed()
    print(result)


asyncio.run(main())
```

The [host contracts](crates/system/helios-mesh/README.md#python-host-api)
describe result handling and gateway connections. The
[Python acceptance stories](crates/system/helios-mesh-python/tests/acceptance/)
exercise native exports, discovery errors, and asynchronous behavior without
physical devices. This pulse example requires the matching board and firmware.

### Use The Same Operation From Rust

Inside an async application using the `helios-mesh` crate:

```rust
use helios_mesh::prelude::*;
use std::time::Duration;

let devices = helios_mesh::discover_type(NodeType::Sidecar).await?;
let device = devices.first().ok_or(helios_mesh::Error::DeviceNotFound)?;
let ConnectedNode::Sidecar(sidecar) = device.connect().await? else {
    unreachable!("discovery rechecks the node type")
};

let operation = sidecar.pulse_gpio(AuxGpioId(1), Duration::from_millis(250)).await?;
let result = operation.completed().await?;
assert!(result.cleanup_error.is_none());
```

See the [Rust host API](crates/system/helios-mesh/README.md#rust-host-api)
for discovery, typed gateway scans, operation ownership, and cancellation.

### Coordinate Two Stepper Channels

The [Rotator sweep](crates/system/helios-mesh-python/examples/rotator_sweep.py)
configures and homes both channels, then runs opposed relative moves. The
[motor tuning example](crates/system/helios-mesh-python/examples/motor_tuning.py)
explores motion settings. Both control physical hardware; read the
[motor setup](crates/firmware/helios-mesh-board-feather-rp2040/README.md#motor-setup)
before running them.

## Firmware

Built-in images target Feather RP2040 boards with RFM95 radios:

| Role | Attached Hardware | Purpose |
| --- | --- | --- |
| Gateway | TFT FeatherWing V2 | USB attachment and LoRa coordination |
| Rotator | Motor FeatherWing | Two stepper channels |
| Sidecar | OLED FeatherWing | Two independent GPIOs on D11 and D12 |

On Linux, build and package one profile:

```sh
mise run build --profile dev
mise run package --profile dev
```

`target/package/dev/linux-x86_64/rust/firmware/helios-mesh-firmware-feather-rp2040.zip` contains all
three UF2 images, their matching ELF files in `debug/`, flashing instructions,
and the Apache-2.0 license.
Individual UF2s also remain in `target/package/dev/linux-x86_64/rust/firmware/` for local use. Packaging
consumes the existing build; it does not compile firmware again.

For a blank board, hold BOOTSEL while connecting USB. Copy the role's UF2 to
its mounted `RPI-RP2` volume, or use the CLI with exactly one such volume present:

```sh
mise run dev:cli -- flash target/package/dev/linux-x86_64/rust/firmware/helios-mesh-node-sidecar.uf2
```

Flash one physical board at a time. An already-running, compatible node can
be selected with `--device <id>` to restart into its bootloader and flash.
Read the [flashing guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md#firmware-packages-and-flashing)
before updating boards across a protocol change.

## Project Status & Roadmap

Helios Mesh is in active development at `0.1.0`. Pin the revision you use and
expect API changes before 1.0. The wire protocol is version **14** and requires
an exact match between host software and every participating board.

Native Linux and Windows CI validates host behavior and wheel imports. Linux
cross-compiles all three firmware images. These checks do not establish
physical RF or electrical behavior, and macOS is not certified by CI.

The release workflow is prepared for GitHub CLI downloads and a combined
firmware archive, PyPI wheels, and publishable Rust libraries. First registry
publication and account setup remain pending; build from source for now.
Releases start at `0.1.0` and then increment only the patch component.

Follow [issues](https://github.com/stokker-technologies/helios-mesh/issues)
for development discussions. GitHub is authoritative;
[GitLab](https://gitlab.com/stokker-technologies/open-source/helios-mesh)
is a one-way mirror with CI disabled.

## Documentation

- [Host Contracts](crates/system/helios-mesh/README.md) — discovery, typed Rust/Python APIs, transport, and operation ownership.
- [Board And Firmware Guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md) — wiring, safety, node authoring, and field diagnostics.
- [Python Examples](crates/system/helios-mesh-python/examples/) — Rotator sweep and tuning workflows.
- [Public Rust Facade](crates/system/helios-mesh/src/lib.rs) — the host and firmware API exports.
- [Wire Contract](crates/common/helios-mesh-protocol/src/lib.rs) — the shared versioned protocol.

## Development Setup

After [Quick Start](#quick-start), choose a repository task:

| Command | Purpose |
| --- | --- |
| `mise run check` | Check Hub conventions, formatting, features, firmware compilation, lints, types, and workflow syntax |
| `mise run build --profile dev` | Build shipping host binaries, wheels, and Linux firmware |
| `mise run build:rust:tests --profile dev` | Build the shared test archive and fixtures |
| `mise run check:python:generated --profile dev` | Verify the built wheel against Python stubs |
| `mise run test` | Run Rust unit, integration, acceptance, and Python acceptance lanes |
| `mise run package --profile dev` | Assemble existing CLI, wheel, and firmware artifacts |

Build outputs live under `target/build/<profile>/<target>/`, package outputs under
`target/package/<profile>/<target>/<language>/`, and reports under
`target/tests/<profile>/<target>/`; the examples use `linux-x86_64`, and Windows
uses `windows-x86_64`. Tests consume the matching test archive from
`mise run build:rust:tests` and the installed native wheel. The root
`.venv` is shared by local tasks, which run sequentially; native CI jobs use
isolated workspaces. The root uv workspace includes `tools/python` and owns
`uv.lock`; the Python distribution keeps its independent manifest and lockfile.
Required public CI uses repository-native tooling.

Task changes target `dev`. The manual `Release / Prepare` job opens
`release/v<version>` into `dev`. After the required checks and release rehearsal,
the review squash-merges into `dev`; `Release / Tag` creates the stable tag,
publishes crates, and fast-forwards `main` to that commit. The tag workflow
publishes and verifies the native packages. Release notes come from Conventional
Commits at tag time. Downloads include the firmware ZIP, host binaries, wheels,
and `SHA256SUMS`; the Linux label on the firmware download names its build host,
while its contents target RP2040.

## Contributing

Bug reports, focused fixes, hardware observations, and examples are welcome.
[Open an issue](https://github.com/stokker-technologies/helios-mesh/issues)
to discuss a change. Include the revision, host platform, node role, attached
hardware, and relevant diagnostics when reporting a device problem.

After completing source changes, run the native candidate proof in order:

```sh
mise run check
mise run build
mise run build:rust:tests
mise run check:python:generated
mise run test
mise run package
mise run release:publish:dry
```

Provider CI runs the native graph on every configured platform.
Background validation keeps Windows child consoles hidden while preserving
command output and exit status. Interactive product commands keep their normal
console behavior.

CI runner overrides live in `stkr.toml`: Rust checks and build producers use
local Linux and Windows runners. Rust coverage and Release Dry, Rehearsal and
Tag also compile crates and use local Linux runners. Artifact-consuming tests
and packaging, other checks, publication and final verification use native
hosted runners with the same tasks.
Ordinary CI jobs default to 120 minutes while individual test deadlines remain
unchanged. Pull requests can write compatible caches within their native merge-ref
scope. New PR and ordinary `dev` commits cancel superseded validation while
release tagging and publication retain their required handoff.

Linux and Windows native checks are both required. Hardware tests and
firmware flashing require an explicit decision and suitable physical setup.

## License

Helios Mesh is licensed under [Apache-2.0](LICENSE).
