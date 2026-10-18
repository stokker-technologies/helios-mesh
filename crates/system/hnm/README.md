# HNM Host Contracts

HNM has one public Rust facade, `hnm`, used by host applications and firmware
authors. Host support is the default feature. Firmware uses
`default-features = false, features = ["firmware"]`. The Python adapter mirrors
the typed host API through PyO3.

## Crate Boundaries

The optional `testing` feature supplies an in-process byte-stream backend under
`hnm::testing`. Host integration stories use it to drive the real framing,
request, typed-handle, and operation logic without USB hardware. The Build task
enables this feature for the host test archive; ordinary host and Python users
use the default features.

| Crate | Responsibility |
| --- | --- |
| `hnm-protocol` | Shared wire types, node metadata, request and response catalogs |
| `hnm` | Public types, USB discovery, typed handles, firmware authoring exports |
| `hnm-macros` | Node metadata, typed route dispatch, job catalog delegation |
| `hnm-core` | Shared dispatcher, jobs, storage, mesh, diagnostics, UI projection |
| `hnm-board-feather-rp2040` | Validated wiring and exclusive board resource grants |
| `hnm-drivers` | Peripheral drivers and motor timing residents |
| `hnm-nodes` | Node definitions, jobs, hardware declarations, firmware entry points |
| `hnm-cli`, `hnm-python` | Thin adapters over `hnm` |

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
| Node types | `hnm-protocol/src/nodes/mod.rs` | `sidecar.rs`, `gateway.rs`, `rotator.rs` |
| Requests and responses | `hnm-protocol/src/wire/mod.rs` | `node.rs`, `gateway.rs`, `motion/mod.rs` |
| Sidecar jobs | `hnm-nodes/src/nodes/sidecar/jobs/mod.rs` | `pulse.rs` |
| Rotator jobs | `hnm-nodes/src/nodes/rotator/jobs/mod.rs` | `turn.rs`, `home.rs`, `stop.rs` |
| FeatherWings | `hnm-board-feather-rp2040/src/wings/mod.rs` | Wing-specific wiring files |

Request and response catalogs group payloads by domain. A new domain is one
registration in each outer enum plus its sibling implementation. There is no
separate capability catalog: the selected node type exposes its typed API,
and firmware routing decides support. An unmatched request returns `Unsupported`.

## Rust Host API

```rust
use hnm::{AuxGpioId, ConnectedNode, NodeType};
use std::time::Duration;

let devices = hnm::discover_type(NodeType::Sidecar).await?;
let device = devices.first().ok_or(hnm::Error::DeviceNotFound)?;
let ConnectedNode::Sidecar(sidecar) = device.connect().await? else {
    unreachable!("the discovery entry rechecks identity and type")
};

let operation = sidecar.pulse_gpio(AuxGpioId(1), Duration::from_millis(250)).await?;
let snapshot = operation.completed().await?;
assert!(snapshot.cleanup_error.is_none());
let status = sidecar.node().status().await?;
```

`discover()` enumerates and handshakes all attached HNM USB devices;
`discover_type(NodeType::Gateway)` filters by the firmware's declared type.
Opening a discovered entry rechecks both its device ID and type.
`SidecarNode::connect`, `RotatorNode::connect`, and `GatewayNode::connect`
accept a `DeviceSelector` for direct selection.

Gateway returns the same typed enum for mesh nodes:

```rust
use hnm::{ConnectedNode, DeviceSelector, GatewayNode, NodeType};

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
import hnm

async def main():
    devices = await hnm.discover(hnm.NodeType.SIDECAR)
    sidecar = await devices[0].connect()
    assert isinstance(sidecar, hnm.SidecarNode)

    operation = await sidecar.pulse_gpio(hnm.AuxGpioId(1), timedelta(milliseconds=250))
    snapshot = await operation.completed()
    assert snapshot.cleanup_error is None
    print(sidecar.id, devices[0].node_type.label)

asyncio.run(main())
```

Use `await hnm.discover(hnm.NodeType.GATEWAY)` to find gateways. For mesh
selection, `await gateway.scan_one(hnm.NodeType.ROTATOR)` followed by
`await gateway.connect_node(summary)` returns a `RotatorNode` instance.
The published stubs express the dynamic connection result as a union of the
three concrete classes, allowing `isinstance` narrowing.


