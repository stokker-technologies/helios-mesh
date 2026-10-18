/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Position-controlled motion verbs and boot-local stepper configuration.

/* === Modules === */

pub mod wire {
    pub use helios_mesh_protocol::motion::{Configuration, Limits, Mechanics, Microsteps, Power, Settings, StepperConfiguration};
}

/* === Re-Exports === */

pub use helios_mesh_protocol::motion::{
    Configuration, Configure, Limits, Mechanics, Microsteps, MoveRelative, MoveTo, MoveTogether, Power, Response, SetHome, Settings,
    StepperConfiguration, Stop,
};
pub use helios_mesh_protocol::motion::{Delta, Description, Kind, Target, Timing};

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports
use crate::{
    Error, Result,
    node::{ChannelId, JournalEvent, MAX_CHANNELS, NodeHandle, Operation},
};
use helios_mesh_protocol::{motion, node, wire as protocol_wire};

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone)]
pub struct Axis {
    node: NodeHandle,
    description: Description,
}

pub struct AxesGuard {
    node: NodeHandle,
    axes: Vec<Axis>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepperConfig {
    pub steps: u16,

    pub microsteps: u8,

    pub reversed: bool,

    pub max_speed: u16,

    pub max_acceleration: u16,

    pub moving_pwm: u8,

    pub holding_pwm: u8,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl StepperConfig {
    fn to_wire(self) -> Result<wire::StepperConfiguration> {
        let microsteps = match self.microsteps {
            4 => wire::Microsteps::Four,
            8 => wire::Microsteps::Eight,
            _ => return Err(Error::InvalidArgument("microsteps must be 4 or 8")),
        };
        Ok(wire::StepperConfiguration {
            mechanics: wire::Mechanics { full_steps: self.steps, microsteps, reversed: self.reversed },
            limits: wire::Limits { speed: self.max_speed, acceleration: self.max_acceleration },
            power: wire::Power { moving: self.moving_pwm, holding: self.holding_pwm },
        })
    }
}

impl Axis {
    pub(crate) const fn new(node: NodeHandle, description: Description) -> Self {
        Self { node, description }
    }

    #[must_use]
    pub const fn id(&self) -> ChannelId {
        self.description.channel
    }

    #[must_use]
    pub const fn description(&self) -> &Description {
        &self.description
    }

    pub async fn turn(&self, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> Result<node::OperationSnapshot> {
        self.start_turn(degrees, speed, acceleration).await?.completed().await
    }

    pub async fn move_to(&self, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> Result<node::OperationSnapshot> {
        self.start_move_to(degrees, speed, acceleration).await?.completed().await
    }

    pub async fn zero(&self) -> Result<node::OperationSnapshot> {
        self.start_zero().await?.completed().await
    }

    pub async fn stop(&self) -> Result<node::OperationSnapshot> {
        self.start_stop().await?.settled().await
    }

    pub async fn start_turn(&self, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> Result<Operation> {
        let delta = self.rotational_delta(degrees)?;
        let operation =
            self.node.request(motion::MoveRelative { channel: self.id(), delta, settings: wire::Settings { speed, acceleration } }).await?;
        Ok(self.node.operation(operation))
    }

    pub async fn start_move_to(&self, degrees: f64, speed: Option<u16>, acceleration: Option<u16>) -> Result<Operation> {
        let target = self.rotational_target(degrees)?;
        let operation =
            self.node.request(motion::MoveTo { channel: self.id(), target, settings: wire::Settings { speed, acceleration } }).await?;
        Ok(self.node.operation(operation))
    }

    pub async fn start_zero(&self) -> Result<Operation> {
        let operation = self.node.request(motion::SetHome { channel: self.id() }).await?;
        Ok(self.node.operation(operation))
    }

    pub async fn start_stop(&self) -> Result<Operation> {
        let operation = self.node.request(motion::Stop { channel: self.id() }).await?;
        Ok(self.node.operation(operation))
    }

    fn rotational_delta(&self, degrees: f64) -> Result<motion::Delta> {
        if self.description.kind != Kind::Rotational {
            return Err(Error::InvalidArgument("turn requires a rotational-kind axis"));
        }
        rotational_delta(degrees)
    }

    fn rotational_target(&self, degrees: f64) -> Result<motion::Target> {
        if self.description.kind != Kind::Rotational {
            return Err(Error::InvalidArgument("move_to requires a rotational-kind axis"));
        }
        rotational_target(degrees)
    }
}

impl AxesGuard {
    pub(crate) const fn new(node: NodeHandle, axes: Vec<Axis>) -> Self {
        Self { node, axes }
    }

    #[must_use]
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    pub async fn turn_together(&self, degrees: &[f64]) -> Result<Vec<node::OperationSnapshot>> {
        let operations = self.start_turn_together(degrees).await?;
        let mut snapshots = Vec::with_capacity(operations.len());
        for operation in operations {
            snapshots.push(operation.completed().await?);
        }
        Ok(snapshots)
    }

    pub async fn start_turn_together(&self, degrees: &[f64]) -> Result<Vec<Operation>> {
        if degrees.len() != self.axes.len() {
            return Err(Error::InvalidArgument("turn_together needs exactly one degrees value per selected axis"));
        }
        let mut wire_moves = heapless::Vec::<motion::MoveTo, MAX_CHANNELS>::new();
        for (axis, &value) in self.axes.iter().zip(degrees) {
            let target = axis.rotational_target(value)?;
            wire_moves
                .push(motion::MoveTo { channel: axis.id(), target, settings: wire::Settings::default() })
                .map_err(|_| Error::InvalidArgument("more axes than motion channels"))?;
        }
        let operations = self.node.request(motion::MoveTogether { moves: wire_moves }).await?;
        Ok(operations.into_iter().map(|operation| self.node.operation(operation)).collect())
    }
}

/* === Public Functions === */

pub(crate) fn axes_bundle(node: &NodeHandle, channels: Vec<Description>) -> AxesGuard {
    let mut ordered = channels;
    ordered.sort_by_key(|description| description.channel.0);
    let axes = ordered.into_iter().map(|description| Axis::new(node.clone(), description)).collect();
    AxesGuard::new(node.clone(), axes)
}

#[must_use]
pub fn timing_of(event: &JournalEvent) -> Option<Timing> {
    if let JournalEvent::MotionCompleted { timing, .. } = event {
        return Some(*timing);
    }
    let (JournalEvent::CapabilityEventQueued { event, .. } | JournalEvent::CapabilityEventReturned { event, .. }) = event else {
        return None;
    };
    match &event.payload {
        protocol_wire::EventPayload::Motion(inner) => match inner {
            motion::Event::PositionReached { timing, .. } => Some(*timing),
            _ => None,
        },
    }
}

pub(crate) fn build_configuration(steppers: &[StepperConfig]) -> Result<wire::Configuration> {
    if steppers.len() > MAX_CHANNELS {
        return Err(Error::InvalidArgument("more stepper configs than motion channels"));
    }
    let mut wire_steppers = heapless::Vec::<wire::StepperConfiguration, MAX_CHANNELS>::new();
    for &config in steppers {
        let converted = config.to_wire()?;
        if !converted.is_valid() {
            return Err(Error::InvalidArgument("stepper configuration is unsafe or internally inconsistent"));
        }
        wire_steppers.push(converted).map_err(|_| Error::InvalidArgument("more stepper configs than motion channels"))?;
    }
    let configuration = wire::Configuration { steppers: wire_steppers };
    if !configuration.is_valid() {
        return Err(Error::InvalidArgument("configuration must define at least one valid stepper"));
    }
    Ok(configuration)
}

/* === Private Functions === */

fn rotational_target(degrees: f64) -> Result<motion::Target> {
    if !degrees.is_finite() {
        return Err(Error::InvalidArgument("degrees must be finite"));
    }
    let normalized = degrees.rem_euclid(360.0);
    // Rounding values immediately below 360 degrees can produce one full
    // turn, so normalize the integer representation once more.
    #[allow(clippy::cast_possible_truncation)]
    let microdegrees = (normalized * 1_000_000.0).round() as i64 % 360_000_000;
    i32::try_from(microdegrees)
        .map(|microdegrees| motion::Target::Rotational { microdegrees })
        .map_err(|_| Error::InvalidArgument("degrees are out of range"))
}

fn rotational_delta(degrees: f64) -> Result<motion::Delta> {
    if !degrees.is_finite() {
        return Err(Error::InvalidArgument("degrees must be finite"));
    }
    let value = degrees * 1_000_000.0;
    if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return Err(Error::InvalidArgument("degrees are out of range"));
    }
    // The finite i32 bounds check above makes the float-to-integer cast exact
    // with respect to the requested whole microdegree representation.
    #[allow(clippy::cast_possible_truncation)]
    Ok(motion::Delta::Rotational { microdegrees: value.round() as i32 })
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use helios_mesh_protocol::node::{ChannelId as WireChannelId, EventId, NodeId, OperationId, RequestId};

    use super::*;
    const NODE: NodeId = NodeId(641_905);
    const CHANNEL_1: WireChannelId = WireChannelId(1);
    const OPERATION: OperationId = OperationId(7);

    #[test]
    fn normalizes_absolute_angles() {
        assert_eq!(rotational_target(720.0).unwrap(), motion::Target::Rotational { microdegrees: 0 });
        assert_eq!(rotational_target(-90.0).unwrap(), motion::Target::Rotational { microdegrees: 270_000_000 });
        assert!(rotational_target(f64::NAN).is_err());
    }

    #[test]
    fn preserves_relative_revolutions() {
        assert_eq!(rotational_delta(720.0).unwrap(), motion::Delta::Rotational { microdegrees: 720_000_000 });
        assert!(rotational_delta(f64::INFINITY).is_err());
    }

    #[test]
    fn timing_reads_fact() {
        let timing = Timing { steps: 40, delayed_steps: 1, ..Default::default() };
        let event = JournalEvent::MotionCompleted {
            channel: CHANNEL_1,
            operation: OPERATION,
            requested: Target::Rotational { microdegrees: 0 },
            actual: Target::Rotational { microdegrees: 0 },
            timing,
        };
        assert_eq!(timing_of(&event), Some(timing));
    }

    #[test]
    fn timing_extracts_timing() {
        let timing = Timing { steps: 22, delayed_steps: 3, ..Default::default() };
        let event = JournalEvent::CapabilityEventReturned {
            peer: NODE,
            request_id: RequestId(1),
            event: protocol_wire::Event {
                id: EventId(1),
                payload: protocol_wire::EventPayload::Motion(motion::Event::PositionReached {
                    channel: CHANNEL_1,
                    operation: OPERATION,
                    requested: Target::Rotational { microdegrees: 0 },
                    actual: Target::Rotational { microdegrees: 0 },
                    timing,
                }),
            },
        };
        assert_eq!(timing_of(&event), Some(timing));
    }

    #[test]
    fn timing_none_event() {
        assert_eq!(timing_of(&JournalEvent::NodeEvicted { node: NODE }), None);
    }

    #[test]
    fn stepper_config_safety() {
        let valid = StepperConfig {
            steps: 200,
            microsteps: 8,
            reversed: false,
            max_speed: 60,
            max_acceleration: 180,
            moving_pwm: 25,
            holding_pwm: 10,
        };
        assert!(build_configuration(&[valid]).is_ok());
        assert!(matches!(build_configuration(&[StepperConfig { microsteps: 3, ..valid }]), Err(Error::InvalidArgument(_))));
        assert!(matches!(build_configuration(&[StepperConfig { max_speed: 0, ..valid }]), Err(Error::InvalidArgument(_))));
        assert!(matches!(build_configuration(&[valid; 5]), Err(Error::InvalidArgument(_))));
    }
}
