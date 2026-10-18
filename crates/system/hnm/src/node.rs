/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Universal node handle: every chassis-answered request, on every node,

/* === Modules === */

/* === Re-Exports === */

pub use hnm_protocol::diagnostics::{
    JOURNAL_PAGE_SIZE, JournalEntry, JournalEvent, JournalPage, JournalSummary, NodeDiagnostics, RequestDiagnostics,
};
pub use hnm_protocol::node::{
    AUX_GPIO_COUNT, AuxGpioAccess, AuxGpioError, AuxGpioId, AuxGpioMode, Bootloader, ChannelId, FirmwareVersion, MAX_CHANNELS,
    NodeDescription, NodeId, NodeInfo, NodeStatus, NodeSummary, OperationId, OperationSnapshot, OperationState,
};
pub use hnm_protocol::satellite::{MAX_SATELLITES, SatelliteName, SatelliteReport, SatelliteState};

/* === Imports === */

// Standard Library Imports
use std::time::Duration;

// Third Party Imports

// Local Imports
use crate::{
    Error, Result,
    gateway::GatewayNode,
    journal,
    motion::{self, AxesGuard, Description, StepperConfig},
    transport::{Connection, DEFAULT_TIMEOUT, EXTENDED_TIMEOUT, UsbDevice, usb_devices},
};
use hnm_protocol::{call::Call, gateway, node};

/* === Constants === */

const COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(50);
const UNREACHABLE_PATIENCE: Duration = Duration::from_secs(30);
const ROSTER_SILENCE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const UNREACHABLE_SILENCE_S: u16 = 12;
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone)]
pub struct NodeHandle {
    connection: Connection,
    id: NodeId,
    remote: bool,
    timeout: Option<Duration>,
}

#[derive(Clone)]
pub struct Operation {
    node: NodeHandle,
    id: OperationId,
}

/* === Enums === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceSelector {
    Only,

    Id(NodeId),

    Port(String),
}

/* === Traits === */

/* === Implementations === */

impl NodeHandle {
    pub(crate) fn new(connection: Connection, id: NodeId, remote: bool) -> Self {
        Self { connection, id, remote, timeout: None }
    }

    pub async fn connect(selector: DeviceSelector) -> Result<Self> {
        let devices = usb_devices()?;
        let device = select_device(devices, &selector)?;
        Self::open(&device).await
    }

    pub(crate) async fn connect_gateway(selector: DeviceSelector) -> Result<Self> {
        let devices = usb_devices()?;
        match select_device(devices.clone(), &selector) {
            Ok(device) => Self::open(&device).await,
            Err(Error::AmbiguousDevices(_)) if selector == DeviceSelector::Only => Self::connect_only_coordinator(devices).await,
            Err(error) => Err(error),
        }
    }

    async fn open(device: &UsbDevice) -> Result<Self> {
        let connection = Connection::open(&device.port).await?;
        Ok(Self::new(connection, device.id, false))
    }

    async fn connect_only_coordinator(mut devices: Vec<UsbDevice>) -> Result<Self> {
        devices.sort_by_key(|device| device.id.0);
        let mut probes = Vec::with_capacity(devices.len());
        for device in &devices {
            probes.push(Self::probe(device).await);
        }
        Self::select_probed(&devices, probes).await
    }

    async fn probe(device: &UsbDevice) -> Option<(Self, Option<NodeDescription>)> {
        let node = Self::open(device).await.ok()?;
        // Probe a clone, not `node` itself: `with_timeout` mutates the
        // handle it is called on, and the kept `node` must keep the default
        // timeout it will serve real requests under once selected. Bound to
        // `described` before constructing the returned tuple so the clone's
        // temporary provably drops right here, leaving `node` the sole
        // owner of its connection for `select_probed`'s eventual
        // `Connection::close`.
        let described = node.clone().with_timeout(PROBE_TIMEOUT).describe().await;
        Some((node, described.ok()))
    }

    pub(crate) async fn select_probed(devices: &[UsbDevice], probes: Vec<Option<(Self, Option<NodeDescription>)>>) -> Result<Self> {
        let descriptions: Vec<_> = probes.iter().map(|probe| probe.as_ref().and_then(|(_, description)| description.clone())).collect();
        let selected = select_coordinator(devices, &descriptions);
        let mut winner = None;
        for (node, _) in probes.into_iter().flatten() {
            match &selected {
                Ok(id) if node.id == *id => winner = Some(node),
                _ => node.close().await,
            }
        }
        selected.map(|_| winner.expect("select_coordinator only returns an id present among the probed candidates"))
    }

    pub(crate) async fn close(self) {
        self.connection.close().await;
    }

    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    #[must_use]
    pub const fn is_remote(&self) -> bool {
        self.remote
    }

    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn request<C: Call>(&self, call: C) -> Result<C::Output> {
        self.call_extended(call, false).await
    }

    async fn call_extended<C: Call>(&self, call: C, extended: bool) -> Result<C::Output> {
        let timeout = self.timeout.unwrap_or(if extended || self.remote { EXTENDED_TIMEOUT } else { DEFAULT_TIMEOUT });
        self.connection.call(self.id, timeout, call).await
    }

    pub async fn describe(&self) -> Result<NodeDescription> {
        self.request(node::Describe).await
    }

    pub async fn status(&self) -> Result<NodeStatus> {
        self.request(node::Status).await
    }

    pub async fn identify(&self) -> Result<()> {
        self.request(node::Identify).await
    }

    pub async fn diagnostics(&self) -> Result<NodeDiagnostics> {
        self.request(node::ReadDiagnostics).await
    }

    pub async fn journal_page(&self, after_sequence: u32) -> Result<JournalPage> {
        self.call_extended(node::ReadJournal { after_sequence }, true).await
    }

    pub async fn journal(&self) -> Result<Vec<JournalEntry>> {
        let mut cursor = journal::Cursor::new();
        let mut entries = Vec::new();
        let mut target = None;
        loop {
            let batch = cursor.next(self).await?;
            let target = *target.get_or_insert(cursor.latest());
            if batch.is_empty() {
                break;
            }
            entries.extend(batch);
            if cursor.position() >= target {
                break;
            }
        }
        Ok(entries)
    }

    pub(crate) async fn configure_aux_gpio(&self, gpio: AuxGpioId, mode: AuxGpioMode) -> Result<()> {
        if !gpio.is_valid() {
            return Err(Error::AuxGpio { gpio, reason: AuxGpioError::InvalidId });
        }
        self.request(node::ConfigureAuxGpio { gpio, mode }).await
    }

    pub(crate) async fn pulse_aux_gpio(&self, gpio: AuxGpioId, duration: Duration) -> Result<Operation> {
        if !gpio.is_valid() {
            return Err(Error::AuxGpio { gpio, reason: AuxGpioError::InvalidId });
        }
        let duration_ms =
            u32::try_from(duration.as_millis()).map_err(|_| Error::AuxGpio { gpio, reason: AuxGpioError::InvalidDuration })?;
        if duration_ms == 0 || duration_ms > node::MAX_AUX_GPIO_PULSE_MS || !duration.subsec_nanos().is_multiple_of(1_000_000) {
            return Err(Error::AuxGpio { gpio, reason: AuxGpioError::InvalidDuration });
        }
        let id = self.request(node::PulseAuxGpio { gpio, duration_ms }).await?;
        Ok(self.operation(id))
    }

    pub async fn restart(&self, bootloader: Option<Bootloader>) -> Result<()> {
        self.request(node::Restart { bootloader }).await
    }

    pub async fn operation_status(&self, operation: OperationId) -> Result<OperationSnapshot> {
        self.request(node::OperationStatus { operation }).await
    }

    pub async fn poll(&self) -> Result<()> {
        self.request(node::Poll).await
    }

    async fn roster_entry(&self) -> Result<Option<NodeSummary>> {
        let after = Some(NodeId(self.id.0.saturating_sub(1)));
        let page: gateway::NodePage = self.request(gateway::Nodes { after }).await?;
        Ok(page.nodes.into_iter().find(|summary| summary.info.id == self.id))
    }

    #[must_use]
    pub fn operation(&self, id: OperationId) -> Operation {
        Operation { node: self.clone(), id }
    }

    pub(crate) fn channels() -> impl core::future::Future<Output = Result<Vec<Description>>> {
        core::future::ready(Ok([1, 2]
            .into_iter()
            .map(|channel| Description { channel: ChannelId(channel), kind: motion::Kind::Rotational, minimum: 0, maximum: 359_999_999 })
            .collect()))
    }

    pub(crate) async fn axes(&self, channels: Option<&[ChannelId]>) -> Result<AxesGuard> {
        let advertised = Self::channels().await?;
        let selected = match channels {
            None => advertised,
            Some(ids) => {
                let mut selected = Vec::with_capacity(ids.len());
                for &id in ids {
                    let description = advertised.iter().find(|description| description.channel == id).copied().ok_or(Error::NotFound)?;
                    selected.push(description);
                }
                selected
            }
        };
        Ok(motion::axes_bundle(self, selected))
    }

    pub(crate) async fn configure(&self, steppers: &[StepperConfig]) -> Result<()> {
        let configuration = motion::build_configuration(steppers)?;
        self.request(hnm_protocol::motion::Configure { configuration }).await
    }

    pub(crate) async fn gateway(&self) -> Result<GatewayNode> {
        if self.describe().await?.info.node_type != crate::NodeType::Gateway {
            return Err(Error::Unsupported);
        }
        Ok(GatewayNode::new(self.clone()))
    }

    pub(crate) fn peer(&self, id: NodeId) -> Self {
        Self::new(self.connection.clone(), id, id != self.id || self.remote)
    }
}

impl Operation {
    pub async fn cancel(&self) -> Result<()> {
        self.node.request(node::CancelOperation { operation: self.id }).await
    }

    #[must_use]
    pub const fn id(&self) -> OperationId {
        self.id
    }

    pub async fn status(&self) -> Result<OperationSnapshot> {
        self.node.operation_status(self.id).await
    }

    pub async fn completed(&self) -> Result<OperationSnapshot> {
        let snapshot = self.settled().await?;
        match snapshot.state {
            OperationState::Stopped => Err(Error::OperationStopped(self.id)),
            _ => Ok(snapshot),
        }
    }

    pub async fn settled(&self) -> Result<OperationSnapshot> {
        let mut unreachable_since: Option<std::time::Instant> = None;
        let mut next_roster_check = self.node.is_remote().then(|| tokio::time::Instant::now() + ROSTER_SILENCE_CHECK_INTERVAL);
        loop {
            // A directly attached node's poll drives its local event queue
            // with cumulative acknowledgment, so completion events surface
            // promptly; the snapshot then reconciles the observed state. A
            // remote node's events already reach the gateway without this
            // call, so only the (zero-air-cost) cached status is read.
            let outcome = async {
                if !self.node.is_remote() {
                    self.node.poll().await?;
                }
                self.status().await
            }
            .await;
            let snapshot = match outcome {
                Ok(snapshot) => {
                    unreachable_since = None;
                    snapshot
                }
                Err(error @ (Error::Unavailable | Error::Timeout)) => {
                    let since = *unreachable_since.get_or_insert_with(std::time::Instant::now);
                    if since.elapsed() > UNREACHABLE_PATIENCE {
                        return Err(error);
                    }
                    tokio::time::sleep(COMPLETION_POLL_INTERVAL).await;
                    continue;
                }
                Err(error) => return Err(error),
            };
            match snapshot.state {
                OperationState::Active => {
                    if let Some(deadline) = next_roster_check {
                        let now = tokio::time::Instant::now();
                        if now >= deadline {
                            next_roster_check = Some(now + ROSTER_SILENCE_CHECK_INTERVAL);
                            self.check_roster_silence().await?;
                        }
                    }
                    tokio::time::sleep(COMPLETION_POLL_INTERVAL).await;
                }
                OperationState::Failed => return Err(Error::OperationFailed(self.id)),
                OperationState::Completed | OperationState::Stopped => return Ok(snapshot),
            }
        }
    }

    async fn check_roster_silence(&self) -> Result<()> {
        match self.node.roster_entry().await {
            Ok(Some(summary)) if summary.silent_for_s > UNREACHABLE_SILENCE_S => {
                Err(Error::NodeUnreachable { node: self.node.id(), silent_for_s: summary.silent_for_s })
            }
            Ok(_) | Err(Error::Unavailable | Error::Timeout) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn select_device(mut devices: Vec<UsbDevice>, selector: &DeviceSelector) -> Result<UsbDevice> {
    devices.sort_by_key(|device| device.id.0);
    match selector {
        DeviceSelector::Id(id) => devices.into_iter().find(|device| device.id == *id).ok_or(Error::DeviceIdNotFound(*id)),
        DeviceSelector::Port(port) => devices.into_iter().find(|device| device.port == *port).ok_or(Error::DeviceNotFound),
        DeviceSelector::Only => match devices.len() {
            0 => Err(Error::DeviceNotFound),
            1 => Ok(devices.remove(0)),
            _ => Err(Error::AmbiguousDevices(devices.iter().map(Into::into).collect())),
        },
    }
}

fn select_coordinator(devices: &[UsbDevice], probes: &[Option<NodeDescription>]) -> Result<NodeId> {
    let coordinators: Vec<&UsbDevice> = devices
        .iter()
        .zip(probes)
        .filter_map(|(device, probe)| {
            let description = probe.as_ref()?;
            (description.info.node_type == crate::NodeType::Gateway).then_some(device)
        })
        .collect();
    match coordinators.len() {
        0 => Err(Error::DeviceNotFound),
        1 => Ok(coordinators[0].id),
        _ => Err(Error::AmbiguousDevices(coordinators.into_iter().map(Into::into).collect())),
    }
}

/* === Tests === */

#[cfg(test)]
mod tests {

    use super::*;
    use crate::testing;

    fn device(id: u32, name: &str) -> UsbDevice {
        UsbDevice { id: NodeId(id), label: name.to_owned(), port: format!("/dev/tty.fake-{id}") }
    }

    fn description(coordinator: bool) -> NodeDescription {
        let mut info = testing::info(NodeId(0));
        info.node_type = if coordinator { crate::NodeType::Gateway } else { crate::NodeType::Sidecar };
        NodeDescription { info, satellites: heapless::Vec::new() }
    }

    #[test]
    fn sole_coordinator_selected() {
        let devices = [device(273_282, "Gateway"), device(566_114, "Rotator")];
        let probes = [Some(description(true)), Some(description(false))];
        assert_eq!(select_coordinator(&devices, &probes).unwrap(), NodeId(273_282));
    }

    #[test]
    fn zero_coordinators_ambiguous() {
        let devices = [device(273_282, "Rotator"), device(566_114, "Sidecar")];
        let probes = [Some(description(false)), Some(description(false))];
        assert!(matches!(select_coordinator(&devices, &probes), Err(Error::DeviceNotFound)));
    }

    #[test]
    fn failed_probe_coordinator() {
        let devices = [device(273_282, "Gateway"), device(566_114, "Dead")];
        let probes = [Some(description(true)), None];
        assert_eq!(select_coordinator(&devices, &probes).unwrap(), NodeId(273_282));
    }

    #[test]
    fn several_coordinators_listed() {
        let devices = [device(111_111, "Gateway A"), device(222_222, "Rotator"), device(333_333, "Gateway B")];
        let probes = [Some(description(true)), Some(description(false)), Some(description(true))];
        let Err(Error::AmbiguousDevices(candidates)) = select_coordinator(&devices, &probes) else {
            panic!("expected an ambiguous-devices error");
        };
        assert_eq!(
            candidates.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![NodeId(111_111), NodeId(333_333)],
            "the non-coordinator candidate must not appear in the ambiguity list"
        );
    }

    #[test]
    fn no_candidates_found() {
        assert!(matches!(select_coordinator(&[], &[]), Err(Error::DeviceNotFound)));
    }
}
