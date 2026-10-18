/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Position-controlled motion capability.

/* === Modules === */

pub mod stepper;

/* === Re-Exports === */

pub use stepper::{Limits, Mechanics, Microsteps, Power, StepperConfiguration};

/* === Imports === */

// Standard Library Imports

// Third Party Imports
use serde::{Deserialize, Serialize};

// Local Imports
use crate::node::{ChannelId, MAX_CHANNELS, OperationId};

/* === Constants === */

/* === Macros === */

wrap_requests!(
    Motion: MoveTo,
    MoveRelative,
    MoveTogether,
    Stop,
    SetHome,
    Configure,
);

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Accepted {
    pub operation: OperationId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedTogether {
    pub operations: heapless::Vec<OperationId, MAX_CHANNELS>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configured;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub speed: Option<u16>,

    pub acceleration: Option<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Description {
    pub channel: ChannelId,

    pub kind: Kind,

    pub minimum: i32,

    pub maximum: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub steppers: heapless::Vec<StepperConfiguration, MAX_CHANNELS>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveTo {
    pub channel: ChannelId,

    pub target: Target,

    pub settings: Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveRelative {
    pub channel: ChannelId,

    pub delta: Delta,

    pub settings: Settings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveTogether {
    pub moves: heapless::Vec<MoveTo, MAX_CHANNELS>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub channel: ChannelId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetHome {
    pub channel: ChannelId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Configure {
    pub configuration: Configuration,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    pub duration_ms: u32,

    pub steps: u32,

    pub delayed_steps: u32,

    pub maximum_lateness_us: u32,

    pub delayed_100_to_250_us: u32,

    pub delayed_250_to_500_us: u32,

    pub delayed_500_to_1000_us: u32,

    pub delayed_over_1000_us: u32,

    pub maximum_consecutive_delayed_steps: u32,

    pub schedule_rebases: u32,

    pub maximum_output_queue_us: u32,

    pub maximum_i2c_update_us: u32,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Rotational,

    Translational,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Target {
    Rotational { microdegrees: i32 },

    Translational { micrometres: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Delta {
    Rotational { microdegrees: i32 },

    Translational { micrometres: i32 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Request {
    MoveTo(MoveTo),

    MoveRelative(MoveRelative),

    MoveTogether(MoveTogether),

    Stop(Stop),

    SetHome(SetHome),

    Configure(Configure),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Accepted { operation: OperationId },

    AcceptedTogether { operations: heapless::Vec<OperationId, MAX_CHANNELS> },

    Configured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Homed { channel: ChannelId, operation: OperationId },

    PositionReached { channel: ChannelId, operation: OperationId, requested: Target, actual: Target, timing: Timing },

    Stopped { channel: ChannelId, operation: OperationId },

    Failed { channel: ChannelId, operation: OperationId },
}

/* === Traits === */

/* === Implementations === */

impl From<Accepted> for crate::wire::Response {
    fn from(value: Accepted) -> Self {
        Response::Accepted { operation: value.operation }.into()
    }
}

impl From<AcceptedTogether> for crate::wire::Response {
    fn from(value: AcceptedTogether) -> Self {
        Response::AcceptedTogether { operations: value.operations }.into()
    }
}

impl From<Configured> for crate::wire::Response {
    fn from(value: Configured) -> Self {
        let _ = value;
        Response::Configured.into()
    }
}

impl Settings {
    #[must_use]
    pub const fn is_valid_for(self, limits: Limits) -> bool {
        matches!(self.speed, None | Some(1..))
            && matches!(self.acceleration, None | Some(1..))
            && match self.speed {
                Some(speed) => speed <= limits.speed,
                None => true,
            }
            && match self.acceleration {
                Some(acceleration) => acceleration <= limits.acceleration,
                None => true,
            }
    }

    #[must_use]
    pub const fn speed(self, limits: Limits) -> u16 {
        match self.speed {
            Some(speed) => speed,
            None => limits.speed,
        }
    }

    #[must_use]
    pub const fn acceleration(self, limits: Limits) -> u16 {
        match self.acceleration {
            Some(acceleration) => acceleration,
            None => limits.acceleration,
        }
    }
}

impl Configuration {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.steppers.is_empty() && self.steppers.iter().all(|stepper| stepper.is_valid())
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits { speed: 360, acceleration: 720 };

    #[test]
    fn omitted_motion_limits() {
        let settings = Settings::default();
        assert!(settings.is_valid_for(LIMITS));
        assert_eq!(settings.speed(LIMITS), 360);
        assert_eq!(settings.acceleration(LIMITS), 720);
    }

    #[test]
    fn requested_motion_limits() {
        assert!(Settings { speed: Some(180), acceleration: Some(360) }.is_valid_for(LIMITS));
        assert!(!Settings { speed: Some(0), acceleration: None }.is_valid_for(LIMITS));
        assert!(!Settings { speed: Some(361), acceleration: None }.is_valid_for(LIMITS));
    }

    #[test]
    fn configurations_define_stepper() {
        let stepper = StepperConfiguration {
            mechanics: Mechanics { full_steps: 200, microsteps: Microsteps::Eight, reversed: false },
            limits: LIMITS,
            power: Power { moving: 25, holding: 10 },
        };
        let empty = Configuration { steppers: heapless::Vec::new() };
        assert!(!empty.is_valid());
        let pair = Configuration { steppers: heapless::Vec::from_slice(&[stepper; 2]).unwrap() };
        assert!(pair.is_valid());
        let broken = Configuration {
            steppers: heapless::Vec::from_slice(&[
                stepper,
                StepperConfiguration { mechanics: Mechanics { full_steps: 0, ..stepper.mechanics }, ..stepper },
            ])
            .unwrap(),
        };
        assert!(!broken.is_valid());
    }
}
