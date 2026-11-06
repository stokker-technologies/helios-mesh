# Helios Mesh Host Contracts

Helios Mesh has one public Rust facade, `helios_mesh`, used by host applications and firmware
authors. Host support is the default feature. Firmware uses
`default-features = false, features = ["firmware"]`. The Python adapter mirrors
the typed host API through PyO3.

## Crate Boundaries

The optional `testing` feature supplies an in-process byte-stream backend under
`helios_mesh::testing`. Host integration stories use it to drive the real framing,
request, typed-handle, and operation logic without USB hardware. The local
`test:rust:extended` task enables this feature; ordinary host and Python users
use the default features.

| Crate | Responsibility |
| --- | --- |
| `helios-mesh-protocol` | Shared wire types, node metadata, request and response catalogs |
| `helios_mesh` | Public types, USB discovery, typed handles, firmware authoring exports |
| `helios-mesh-macros` | Node metadata, typed route dispatch, job catalog delegation |
| `helios-mesh-core` | Shared dispatcher, jobs, storage, mesh, diagnostics, UI projection |
| `helios-mesh-board-feather-rp2040` | Validated wiring and exclusive board resource grants |
| `helios-mesh-drivers` | Peripheral drivers and motor timing residents |
| `helios-mesh-nodes` | Node definitions, jobs, hardware declarations, firmware entry points |
| `helios-mesh-cli`, `helios-mesh-python` | Thin adapters over `helios_mesh` |

## Identity And Catalogs

`NodeInfo` contains `id`, `node_type`, and `firmware`. Each `NodeType` resolves a
single `NodeDefinition`: `identifier` is machine-readable, `label` is always
human-readable, and `description` explains the node. For example, Gateway uses
`identifier = "hnm.gateway"` and `label = "Gateway"`.

Node types serialize by identifier, never enum position. A fork may reorder
its node catalog without changing those identities. Unknown identifiers are
rejected: a host must include the definitions for the firmware it controls.
Request and response enums still use their wire encoding, so changes to them
require a protocol version change and matching host/firmware builds.

The Rust catalog locations are:

| Catalog | Registration | Implementations |
| --- | --- | --- |
| Node types | `helios-mesh-protocol/src/nodes/mod.rs` | `sidecar.rs`, `gateway.rs`, `rotator.rs` |
| Requests and responses | `helios-mesh-protocol/src/wire/mod.rs` | `node.rs`, `gateway.rs`, `motion/mod.rs` |
| Sidecar jobs | `helios-mesh-nodes/src/nodes/sidecar/jobs/mod.rs` | `pulse.rs` |
| Rotator jobs | `helios-mesh-nodes/src/nodes/rotator/jobs/mod.rs` | `turn.rs`, `home.rs`, `stop.rs` |
| FeatherWings | `helios-mesh-board-feather-rp2040/src/wings/mod.rs` | Wing-specific wiring files |

Request and response catalogs group payloads by domain. A new domain is one
registration in each outer enum plus its sibling implementation. There is no
separate capability catalog: the selected node type exposes its typed API,
and firmware routing decides support. An unmatched request returns `Unsupported`.

## Rust Host API

Use `helios_mesh::prelude::*` for shared protocol types and macros, host discovery
and typed handles, and firmware authoring types when their features are enabled.
Board and wing types additionally require an ARM target and the `firmware` feature.
The same items remain available through their existing crate paths.

```rust
use helios_mesh::prelude::*;
use std::time::Duration;

let devices = helios_mesh::discover_type(NodeType::Sidecar).await?;
let device = devices.first().ok_or(helios_mesh::Error::DeviceNotFound)?;
let ConnectedNode::Sidecar(sidecar) = device.connect().await? else {
    unreachable!("the discovery entry rechecks identity and type")
};

let operation = sidecar.pulse_gpio(AuxGpioId(1), Duration::from_millis(250)).await?;
let snapshot = operation.completed().await?;
assert!(snapshot.cleanup_error.is_none());
let status = sidecar.node().status().await?;
```

`discover()` enumerates and handshakes all attached Helios Mesh USB devices;
`discover_type(NodeType::Gateway)` filters by the firmware's declared type.
Opening a discovered entry rechecks both its device ID and type.
`SidecarNode::connect`, `RotatorNode::connect`, and `GatewayNode::connect`
accept a `DeviceSelector` for direct selection.

Gateway returns the same typed enum for mesh nodes:

```rust
use helios_mesh::prelude::*;

let gateway = GatewayNode::connect(DeviceSelector::Only).await?;
let summary = gateway.scan_one(Some(NodeType::Rotator)).await?;
let ConnectedNode::Rotator(rotator) = gateway.connect_node(summary).await? else {
    unreachable!()
};
let channels = rotator.channels().await?;
```

`NodeHandle` exposes universal requests and the typed `request<C: Call>` escape
hatch. GPIO methods belong to `SidecarNode`; motion configuration and axes belong
to `RotatorNode`; mesh scans belong to `GatewayNode`. There are no legacy `Node`
or `Gateway` host aliases.

## Python Host API

```python
import asyncio
from datetime import timedelta
import helios_mesh

async def main():
    devices = await helios_mesh.discover(helios_mesh.NodeType.SIDECAR)
    sidecar = await devices[0].connect()
    assert isinstance(sidecar, helios_mesh.SidecarNode)

    operation = await sidecar.pulse_gpio(helios_mesh.AuxGpioId(1), timedelta(milliseconds=250))
    snapshot = await operation.completed()
    assert snapshot.cleanup_error is None
    print(sidecar.id, devices[0].node_type.label)

asyncio.run(main())
```

Use `await helios_mesh.discover(helios_mesh.NodeType.GATEWAY)` to find gateways. For mesh
selection, `await gateway.scan_one(helios_mesh.NodeType.ROTATOR)` followed by
`await gateway.connect_node(summary)` returns a `RotatorNode` instance.
The published stubs express the dynamic connection result as a union of the
three concrete classes, allowing `isinstance` narrowing.

## Device Selection

The CLI uses one command tree. With exactly one node attached over USB, that
node is selected automatically. With several attached, the single Gateway
takes precedence; otherwise select the attachment with `--device <id>` or
`--port <serial-port>`. IDs have six decimal digits. `--node <id>` addresses a
remote node through the selected Gateway; without it, commands address the
attached node itself.

```sh
mise run dev:cli -- --device 273282 status
mise run dev:cli -- --device 273282 --node 566114 status
mise run dev:cli -- --port /dev/ttyACM1 status
```

Ambiguous-device errors list the attached nodes. Remote addressing requires a
Gateway, and gateway brokerage commands such as `nodes` remain gateway-only.
USB product strings retain the `Helios Node Mesh` prefix and node identifiers
retain `hnm.*`, preserving discovery and stored identities across the rename.

On Linux, grant the current user access to the serial device. Fedora and Debian
family systems commonly use `sudo usermod -aG dialout $USER`, followed by a new
login. Windows exposes the node as a COM port without an additional driver.

The Python distribution is named `helios-mesh` and imports as `helios_mesh`.
Build with `mise run build --profile dev`; `mise run dev:python` installs the
extension for local development. Wheels are ABI3 packages for each native host.
Stable releases publish Rust libraries to crates.io, ABI3 wheels to PyPI, and CLI and firmware downloads to GitHub.
Protected `ci.yml` jobs use trusted publishing in the `release` environment.
Source builds do not require a published package.

## Operation Ownership

`RotatorNode` owns `configure` and `axes`; `SidecarNode` owns `set_gpio` and
`pulse_gpio`. Python operations are native awaitables, so independent channels
can run with `asyncio.gather`. `operation.cancel()` requests cleanup without
terminating the worker. Disconnecting the host does not cancel admitted work.
Execution and cleanup failures remain distinct in operation snapshots.

Universal requests live on the typed handle's `node()` accessor in Rust and
`node` property in Python. `NodeHandle` provides universal access directly.
Firmware resources and bounded queues belong to their initialized channels;
the node macro supplies worker tasks and permanent storage. Completion futures
compose with Embassy `join`.

## Transport And Diagnostics

USB has one framing reader and writer and correlates concurrent requests by
request ID. Callers do not hold a connection-wide lock while awaiting responses;
completed requests can return out of order. A corrupt frame costs one frame,
not the connection. The half-duplex radio resident exclusively owns the RFM95
and executes one selected transaction at a time.

LoRa exchanges use a random boot-session ID, a 32-bit request ID, three bounded
interactive attempts, a fixed low-latency retry interval, and receiver-side
response replay. Retries reuse the same session/request pair. Background
discovery and maintenance use one short attempt rather than a foreground retry
budget. Long-running requests return an operation ID on admission; status reads
the worker's completion state, including cleanup errors.

### Remote Readiness And Refusals

`GatewayNode::connect` verifies the USB gateway only. `remote(id)` constructs an
addressed handle without probing it; `SidecarNode::from_handle` sends `Describe`
and verifies the returned type. Neither gateway connection nor a cached roster
entry guarantees a live radio exchange. `scan` reads that cached roster.

For a registered node, an exhausted radio exchange returns `Unavailable`; an
unsuccessful probe of an unknown ID returns `NotFound`. Three 200 ms receive
windows with 20 ms between attempts can therefore refuse a remote conversion
within roughly a second, before the host's 20-second remote timeout. A malformed
packet is discarded within the same receive window; unrelated packets do not
restart its deadline. Radio driver failures can also abort an exchange.

Retrying a read such as `Describe` is safe, within the caller's existing overall
deadline. A new mutation call uses a new request ID: do not blindly retry an
uncertain admission, because its physical work may already have started. Firmware
retries retain the original ID and replay the response. `Unavailable` also covers
missing or faulted channel resources, failed configuration persistence, motor
driver failure or completion timeout, job deadline expiry, and exhaustion of a
channel's 24-bit operation sequence until reboot. Repeated calls cannot repair
every cause. A stale roster entry can persist until both eight consecutive
failures and four minutes of silence justify eviction.

There is no fleet-readiness API today. Prefer a future explicit, deadline-bounded
remote probe over waiting for the entire fleet inside USB gateway connection:
the fleet is dynamic, and one absent leaf must not delay local gateway use.
Such an API would establish only that one node answered a read at that instant.

### Multi-Node Bounds

The Gateway has capacity for 16 registered nodes and 32 pending radio jobs,
including two emergency slots. Selection orders emergency, interactive,
operation, maintenance, and discovery work, then deadline and least-recently
served node. Maintenance deadlines are staggered over thirty seconds; an
arriving host request takes precedence over new background work. Node listings
are paged in stable ID order. Best-effort broadcast discovery continues while
nodes are registered so newly powered nodes can join without a Gateway restart.

These are shared bounds, not per-node credits: the host allows 16 in-flight USB
requests, but firmware retains only eight pending forwards. Excess forwards or
scheduled work return `Busy`. Firmware has eight input slots, eight USB response
slots with oldest-response eviction, six cached radio replies per node, and one
mesh reply slot for its single radio resident. Nearby/UI observations hold 16
peers. Each job channel defaults to four queued jobs and 128 history records;
the gateway tracks 64 operations across leaves. The two motor/GPIO channels and
two emergency slots are not two-peer limits.

Radio routing is a gateway-to-leaf star on one channel, not multi-hop relaying.
Each leaf has its own request sequence at the gateway; duplicate detection uses
source ID, source boot session, and request ID at the receiving node. Distinct
physical boards must have distinct six-digit IDs; there is no duplicate-ID
negotiation. Write binding belongs to each leaf, so adding another leaf does not
take another leaf's binding. `BoundElsewhere` rejects writes from another gateway;
reads, including the typed-handle `Describe`, remain open.

The deterministic firmware harness in `helios-mesh-core/tests/jobs/mesh.rs` drives
the production gateway dispatcher, coordinator, link, and codec with scheduled
chip-level packet arrivals. It covers 2, 3, 4, 8, and 17 total nodes, roster paging,
addressing, lost-reply replay, queue saturation, malformed frames, and receive
deadlines. Host transport coverage checks concurrent typed handles and reversed
responses at the same sizes. These tests prescribe packet delivery; they do not
model RF capture, airtime overlap, signal strength, or board scheduling.

Transport establishes request origin; caller metadata does not establish it.
Gateway brokerage arriving over radio is rejected before replay and dispatch.
Ordinary node reads remain open over radio; mutations use the firmware's
[write binding](../../firmware/helios-mesh-board-feather-rp2040/README.md#radio-write-binding).
A radio source ID is a transport identity, not a cryptographically authenticated
principal. USB bypasses this radio write binding.

`diagnostics` projects the selected node's live request counters; `events` reads
its bounded in-memory journal. `record journal.jsonl` follows events into the
versioned `helios_mesh::journal` record schema. Recording does not await or fill
the behavioral mailbox. Journal sequence numbers and boot-relative timestamps
are local to each node; compare intervals and ordering rather than absolute
timestamps between boards. Capture evidence before rebooting.

`restart` reboots the selected attached node; `--node <id> restart` forwards
the request and follows radio write-binding admission. `--bootloader` enters
the selected node's USB BOOTSEL mode.

Connection requests include queueing, USB writing and response waiting in their existing deadline.
A stalled or partially written USB frame terminates the driver and its reader, releasing the transport; reconnect before issuing another command.
GPIO writes and pulses are never replayed automatically after an uncertain acknowledgement.
Host `tracing` events report the node, request type, remote route, deadline and failure; gateway probe events also name the USB port.
The application owns subscriber installation and capturing those messages. Serial-open errors include the port in their returned error.
