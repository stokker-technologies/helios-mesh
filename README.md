# Helios Mesh

Helios Mesh is a Rust workspace for RP2040 node firmware and the
system software that discovers and controls those nodes.

Gateway, Rotator, and Sidecar expose typed Rust and Python APIs over USB and
through a gateway over LoRa. Gateway declares a TFT FeatherWing, Rotator a Motor
FeatherWing, and Sidecar an OLED FeatherWing with two independent GPIOs.

Nodes have a stable `NodeType` with `identifier`, human-readable `label`, and
`description`. Rust catalogs register nodes, requests, responses, jobs, and wings.
Typed routes decide support; there is no capability catalog. Long operations are
self-contained jobs submitted to `Channel<Jobs, Resources>` fields. Channels own
their user-authored resources and bounded queues, with explicit cancellation and
cleanup that preserves the original result. The node macro supplies permanent
storage and worker tasks; completion futures compose with Embassy `join`.

The tested motor stack is Adafruit Motor FeatherWing product 2927 at I2C
address `0x60` with Adafruit product 324 12 V, 350 mA, 200-step NEMA-17
motors. Motor power belongs on the FeatherWing motor-power terminal. For the
current boot, each channel holds its microstep mode, absolute speed and
acceleration limits, movement and hold duty, and electrical direction; the
host pushes those definitions after connecting. Individual operations may
request lower speed or acceleration without changing the active safety
limits. Position is an open-loop estimate; without encoders or limit switches
the physical position at boot is unknown, and each channel must be homed
before movement after every restart.

Each board derives a memorable six-digit node ID from its onboard flash
unique ID at startup. The ID survives firmware updates and requires no
provisioning.

Every node uses the onboard NeoPixel for the same at-a-glance status:

- slow blue breathing: ready
- brief, dim cyan tick: USB or LoRa protocol communication
- three magenta pulses: identify command

Each firmware image declares its attached hardware before bus startup. The
board validates pin reservations, rejects conflicting wings, and only then
constructs the permitted peripherals. Gateway declares a TFT FeatherWing V2,
Rotator declares only its Motor FeatherWing, and Sidecar declares an SH1107
OLED FeatherWing. There is no automatic fallback between displays. A failed
presence check retains the wing's pin reservations. The Rust `wings::Wing`
catalog records each supported wing and its wiring.

Sidecar allocates GPIO 1 and 2 to D11 and D12, initially unpulled inputs (high
impedance). Both permit high impedance, push-pull high and push-pull low;
outputs use 2 mA drive strength. GPIO IDs 3 and 4 (D24/D25) remain unallocated.
Gateway and Rotator allocate no auxiliary outputs. Status reports allocation
separately from the last applied mode; unallocated or reserved pins reject
all GPIO commands without changing the pads.

```sh
# Build Gateway + TFT, Rotator + Motor, and Sidecar + OLED.
mise run build
mise run package --profile dev
# Use the connected gateway to control a sidecar.
mise run dev:cli -- --node <sidecar-id> gpio 1 high
mise run dev:cli -- --node <sidecar-id> gpio 2 low
mise run dev:cli -- --node <sidecar-id> gpio 1 high-z
```

Declarations must match the complete attached stack and any jumper changes.
For an unknown attachment, `mise run build --hardware-unknown` builds a
USB-only diagnostic image with no shared-header buses, probes or GPIO outputs.
Firmware cannot contain arbitrary electrical faults or guarantee safety
before it runs; see [Hardware and GPIO Safety](crates/firmware/helios-mesh-board-feather-rp2040/README.md#hardware-and-gpio-safety).

Node implementations expose initialized channels through `Node::channels()`; typed
fields retain job submission. `NODE_TYPE` selects catalog metadata while the runtime
owns the device-specific `NodeId`.

## Workspace

```text
crates/
  common/                       cross-runtime contracts only, never generic helpers
    helios-mesh-protocol/               single wire contract shared by firmware and hosts
  system/
    helios-mesh/                        main Rust API facade
    helios-mesh-macros/                 node, route, and job authoring macros
    helios-mesh-cli/                    thin command-line adapter
    helios-mesh-python/                 typed PyO3 adapter and Python examples
  firmware/
    helios-mesh-core/                   shared dispatcher, Node contract, job workers,
                                storage, mesh modes, transport loops, UI model
    helios-mesh-drivers/                embedded-hal wing and display drivers, shared buses
    helios-mesh-board-feather-rp2040/   Feather RP2040 hardware construction only
    helios-mesh-nodes/                  node definitions, job catalogs, and composition roots
```

The `helios-mesh-cli` and `helios-mesh-python` interaction layers consume the public `helios_mesh`
facade. The [host contracts](crates/system/helios-mesh/README.md) describe discovery,
typed node handles, operation ownership, and Python behavior. The
[board and firmware guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md)
covers wiring, GPIO safety, node authoring, and field diagnostics.

The wire protocol is version **14**, with exact-match validation in both
directions. Encoding changes require a protocol bump and matching host and
firmware builds on every participating board. Cargo packages and firmware
currently report `0.1.0`; firmware derives that version from Cargo. Release
preparation keeps all packages synchronized and increments only the patch
component after the first stable release.

## Getting Started

Everything runs through [mise](https://mise.jdx.dev), which provisions the
same toolchain set (Rust, Python, uv, elf2uf2-rs) on Linux, macOS, and
Windows. Install it once:

```sh
curl https://mise.run | sh    # Linux and macOS
winget install jdx.mise       # Windows (or: scoop install mise)
```

Then clone, trust the repository configuration, and install the toolchains:

```sh
git clone https://github.com/stokker-technologies/helios-mesh.git
cd helios-mesh
mise trust
mise run prepare
mise run build
```

`rust-toolchain.toml` pins the stable channel with the `thumbv6m-none-eabi`
ARM target, and the mise rust tool installs that target alongside the
toolchain, so no manual `rustup target add` is needed on any OS. Validate
the checkout:

```sh
mise run check
mise run build
mise run build:tests
mise run package --profile dev
```

Serial access: on Fedora and Debian family systems, add yourself to the
`dialout` group (`sudo usermod -aG dialout $USER`, then log in again) so the
CLI can open `/dev/ttyACM*`. Windows needs no driver or group; the node
appears as a COM port.

Flashing has two supported entry paths, one board at a time:

- A board already running Helios Mesh firmware:
  `mise run dev:cli -- --device <id> restart --bootloader` restarts it into its
  `RPI-RP2` BOOTSEL volume.
- A blank board: hold BOOTSEL while plugging in USB.

Flash an existing Linux package image with `mise run dev:cli -- flash target/package/dev/linux-x86_64/rust/firmware/helios-mesh-node-gateway.uf2` (or the Rotator/Sidecar image). This requires physical access to the selected board. The
underlying `helios-mesh flash` command discovers the
mounted BOOTSEL volume on all three operating systems and refuses to write
unless exactly one volume is present. Alternatively,
`helios-mesh --device <id> flash <uf2>` performs the bootloader restart, volume
wait, copy, and post-flash `status` confirmation in one step.

Verify the flashed node:

```sh
mise run dev:cli -- status
mise run dev:cli -- nodes
mise run dev:cli -- --node <id> status
```

## Commands

```sh
mise run prepare
mise run build
mise run check
mise run build
mise run build:tests
mise run package --profile dev
mise run dev:cli -- flash target/package/dev/linux-x86_64/rust/firmware/helios-mesh-node-rotator.uf2
mise run dev:cli -- flash target/package/dev/linux-x86_64/rust/firmware/helios-mesh-node-gateway.uf2
mise run dev:cli -- flash target/package/dev/linux-x86_64/rust/firmware/helios-mesh-node-sidecar.uf2

mise run dev:cli -- describe
mise run dev:cli -- status
mise run dev:cli -- diagnostics
mise run dev:cli -- events
mise run dev:cli -- identify
mise run dev:cli -- gpio 1 high-z
mise run dev:cli -- gpio 1 high
mise run dev:cli -- gpio 1 low
mise run dev:cli -- move 1 90
mise run dev:cli -- restart
mise run dev:cli -- restart --bootloader
mise run dev:cli -- record journal.jsonl

mise run dev:cli -- nodes
mise run dev:cli -- --node 566114 describe
mise run dev:cli -- --node 566114 status
mise run dev:cli -- --node 566114 move 1 -90
mise run dev:cli -- bench rtt --seconds 60

mise run dev:python
mise run test:integration:python
mise run test:generated
mise run build --profile dev
mise exec -- uv run --locked --project tools/python python crates/system/helios-mesh-python/examples/rotator_sweep.py \
  --device 273282 --node 566114
```

The CLI is one command tree. When exactly one Helios Mesh node is attached over USB
it is selected automatically, and with several attached the single node
declaring the Gateway node type wins; otherwise select one by
stable ID or port with the global `--device` or `--port` flag. The global
`--node` flag addresses a remote node through the attached gateway; without
it, commands address the attached node itself.

USB product strings retain the `Helios Node Mesh` prefix, and node identifiers
retain `hnm.*`, so discovery and stored identities remain stable across the rename.

```sh
mise run dev:cli -- --device 273282 status
mise run dev:cli -- --device 273282 --node 566114 status
mise run dev:cli -- --port /dev/ttyACM1 status
```

Without a selector, an ambiguous-device error lists every attached Helios Mesh node.
Addressing a remote node requires the attached device to be a Gateway. Node diagnostics stay readable over that mesh path,
and restart is admitted through the node's radio write binding. Gateway
brokerage operations such as `nodes` remain attached-gateway-only.

## Typed Host API

```rust
use helios_mesh::{AuxGpioId, ConnectedNode, NodeType};
use std::time::Duration;

let devices = helios_mesh::discover_type(NodeType::Sidecar).await?;
let device = devices.first().ok_or(helios_mesh::Error::DeviceNotFound)?;
let ConnectedNode::Sidecar(sidecar) = device.connect().await? else { unreachable!() };
let operation = sidecar.pulse_gpio(AuxGpioId(1), Duration::from_millis(250)).await?;
let result = operation.completed().await?;
assert!(result.cleanup_error.is_none());
```

```python
import asyncio
from datetime import timedelta
import helios_mesh

async def main():
    devices = await helios_mesh.discover(helios_mesh.NodeType.SIDECAR)
    sidecar = await devices[0].connect()
    assert isinstance(sidecar, helios_mesh.SidecarNode)
    operation = await sidecar.pulse_gpio(helios_mesh.AuxGpioId(1), timedelta(milliseconds=250))
    result = await operation.completed()
    assert result.cleanup_error is None

asyncio.run(main())
```

Filter gateways with `helios_mesh::discover_type(NodeType::Gateway)` in Rust or
`await helios_mesh.discover(helios_mesh.NodeType.GATEWAY)` in Python. Gateway scans accept a
node-type filter and `connect_node` returns the corresponding typed handle.
Universal requests live on each typed handle's `node()` accessor in Rust and
`node` property in Python. `NodeHandle` is also available for universal access.
There are no legacy host `Node` or `Gateway` aliases.

`RotatorNode` owns `configure` and `axes`; `SidecarNode` owns `set_gpio` and
`pulse_gpio`. Python operations are native awaitables, so independent motors
can run with `asyncio.gather`. Execution and cleanup errors remain distinct in
operation snapshots. `operation.cancel()` requests cleanup without terminating
the worker. Disconnecting the host does not cancel admitted work.

See [Host Contracts](crates/system/helios-mesh/README.md) for gateway
and discovery examples, and [Authoring A Node](crates/firmware/helios-mesh-board-feather-rp2040/README.md#authoring-a-node)
for the implemented Sidecar, macros, resources, and job lifecycle. The Python
sweep and tuning examples live under `crates/system/helios-mesh-python/examples/`.

The Python package is built as an ABI3 wheel for each native host. After a
local build, install its wheel with:

```sh
uv pip install target/build/dev/linux-x86_64/python/*.whl
```

The release workflow supports PyPI publication from protected stable tags.
Registry bootstrap and the first publication require separate authorization;
this migration does not publish a package. Unreleased source is available on
`dev`:

```sh
uv add "helios-mesh @ git+https://github.com/stokker-technologies/helios-mesh@dev#subdirectory=crates/system/helios-mesh-python"
```

Each flash task writes only when exactly one mounted `RPI-RP2` BOOTSEL
volume is present. Select and flash one physical role at a time. Both RFM95
boards must have suitable antennas attached before powering radio firmware.

The proof-of-concept P2P link uses 869.525 MHz, spreading factor 7, 250 kHz
bandwidth, coding rate 4/5, and 10 dBm output power. This configuration
assumes operation in Liechtenstein; deployment elsewhere requires checking
the local spectrum rules.

LoRa requests use a random boot-session ID, a 32-bit request ID, three
bounded interactive send attempts, a fixed low-latency retry interval, and
receiver-side response replay. A retry therefore resends the same
boot-session/request pair and cannot execute a command twice. Background
discovery and maintenance use a single short attempt so they cannot occupy
the radio with a foreground retry budget. The first radio write binds the
whole node to one gateway and boot session; reads stay open to every gateway,
the same gateway's rebooted session can take over immediately, and a different
gateway can write after thirty seconds of holder silence. USB requests bypass
the binding. Long-running requests return an operation ID on admission. The worker owns
completion state; `OperationStatus` reads it directly, including cleanup errors.

The gateway coordinator has fixed capacity for 16 registered nodes and 32
pending radio jobs, including two slots reserved for emergency work. It
selects by emergency/interactive/operation/maintenance/discovery class, then
deadline, then least-recently-served node. Idle maintenance deadlines are
deterministically staggered across a thirty-second window, while an arriving
host request wins before new background work begins. Maintenance reads
dynamic status, and node listings are paged in stable node-ID order.

USB requests are framed by one dedicated reader and writer and correlated by
request ID. Concurrent callers may therefore submit work without holding a
connection-wide lock, and the gateway may return completed local or
scheduled requests out of order. The half-duplex radio resident remains the
single owner of the RFM95 and executes one selected transaction at a time.
A corrupt USB frame costs one frame, not the connection.

The gateway repeats best-effort broadcast discovery every few seconds even
while nodes are registered, allowing a newly powered node to appear without
restarting the gateway. Dense groups can still collide because autonomous
response slots are not claimed by this protocol. For deterministic
enrollment, issue `--node <known-id> describe`: the gateway sends that
request by unicast, registers the validated description, and includes the
node in subsequent listings. This registry is in-memory, so provisioning
must be replayed after a gateway restart.

Every gateway request is also a contention-free one-response uplink grant:
the addressed node may piggyback its oldest pending event, and no node
transmits unsolicited traffic. Autonomous node-originated slots require a
shared time epoch and are intentionally not claimed by this implementation.

Decoded requests are paired with context established by the USB or mesh
transport rather than metadata supplied by the caller. The shared chassis
rejects gateway-brokerage requests arriving over radio before replay lookup
and role behavior. Every node request is legal from both origins: reads such
as diagnostics stay open, while mutating requests such as restart proceed
through the radio write-binding gate. A mesh source ID is currently a
transport identity, not a cryptographically authenticated principal;
authentication can later promote it before authorization.

Every node dispatcher owns ephemeral diagnostics for its boot session: live
request counters and a bounded in-memory journal with monotonic sequence
numbers and boot-relative timestamps. Recording is a short synchronized
operation, so observability cannot await or fill the behavioral mailbox. Use
`diagnostics` for the selected node's live projection, locally or with
`--node` across the mesh; use `events` for its retained journal, or `record`
to follow that journal into a JSONL file using the versioned record schema
from `helios_mesh::journal`. Journal timestamps from different boards have
independent boot-time origins, so compare intervals and ordering rather than
absolute values.

Run `mise run dev:cli -- restart` to reboot the selected directly attached node,
or add `--node <id>` to forward the restart through its coordinator. A remote
restart is a radio write and therefore subject to the node's write binding.
Add `--bootloader` to enter the selected node's `RPI-RP2` USB bootloader.

## Repository Tooling

[mise](https://mise.jdx.dev) owns the toolchain and repository recipes.
`mise run prepare` installs the locked tooling environment at the repository
root `.venv`; `mise run build` builds shipping host artifacts and wheels, and
additionally cross-compiles RP2040 firmware on Linux. `mise run build:tests`
then builds the shared test archive and fixtures. Packaging consumes only the
matching Product Build and preserves each platform's native wheel. Tests consume
the shared archive and install the matching wheel.

Build outputs live under `target/build/<profile>/<target>/`; package outputs live
under `target/package/<profile>/<target>/<language>/`. The examples above use
`linux-x86_64`; Windows outputs use `windows-x86_64`. Profiles are `dev` and
`release`. Reports live under `target/tests/<profile>/<target>/`.

Development builds optimize dependencies while leaving workspace crates
unoptimized. Release builds retain firmware symbols for hardware diagnostics.

| Command | Purpose |
| --- | --- |
| `mise run format` | Format maintained Rust, Python, and configuration |
| `mise run check` | Check Hub conventions, formatting, Rust features/firmware, lints, types, and workflow syntax |
| `mise run build --profile dev` | Build shipping host binaries, wheels, and Linux firmware |
| `mise run build:tests --profile dev` | Build the shared test archive and fixtures |
| `mise run test` | Run the declared unit, integration, acceptance, and binding lanes |
| `mise run package --profile dev` | Assemble existing CLI, wheel, and UF2 artifacts |
| `mise run ci` | Run the complete local candidate graph |
| `mise run dev:cli -- --help` | Discover CLI commands |
| `mise run dev:python` | Install the extension for interactive Python development |
| `mise run housekeeping:stats` | Count source lines |

Local mise tasks run sequentially because Python validation shares the root
`.venv` and installs the selected native wheel. This keeps wheel installation
from changing files used by another lane. Native CI jobs use isolated workspaces
and retain their declared parallel scheduling.

GitHub is authoritative; GitLab receives a one-way mirror with CI disabled.
The unified GitHub workflow requires native Linux and Windows host validation,
with RP2040 cross-compilation on Linux. Pull requests run the complete candidate
graph, including unsigned development packages and credential-free publication
dry runs, as do development pushes. Nightly coverage uses the trusted local
runner. Physical RF and electrical behavior and macOS are not certified by CI.

`dev` contains integrated development. Stabilize a release on
`release/v<version>`, prepare synchronized versions and changelog through
`mise run release:prepare`, and merge into `main` preserving ancestry. Incorporate
that history into `dev`. The initial stable version is `0.1.0`; subsequent
releases use patch-only git-cliff calculation, including breaking and feature
commits. There are no prereleases. Only separately authorized protected stable
tags run release-profile packaging and registry/provider publication.

Before the first publication, maintainers must configure crates.io ownership for
the publishable workspace libraries and a PyPI project for `helios-mesh`, then
configure GitHub trusted publishing for this repository's `ci.yml` workflow and
protected `release` environment. First-publication account setup and any package
name registration require separate maintainer authorization.

Project guardrails and skeleton authoring use the local Stokker CLI; required
public CI uses repository-native tools without that private CLI. Trust project
hooks through the agent's native trust flow. Private specifications and machine
state remain ignored; canonical project guidance is maintained separately.

## License

Licensed under the [Apache License 2.0](LICENSE-APACHE).
