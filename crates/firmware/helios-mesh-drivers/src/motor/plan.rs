/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Portable position and trapezoidal-profile calculations for stepper axes.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

const MICRODEGREES_PER_REVOLUTION: i64 = 360_000_000;
const FULL_TURN_MICRODEGREES: i32 = 360_000_000;
const HALF_TURN_MICRODEGREES: i32 = 180_000_000;

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plan {
    direction: Direction,
    total_steps: u32,
    completed_steps: u32,
    target_steps: i32,
    target_microdegrees: i32,
    maximum_steps_per_second: u32,
    acceleration_steps_per_second_squared: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Axis {
    steps_per_revolution: u32,
    position_steps: i32,
}

/* === Enums === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Positive,

    Negative,
}

/* === Traits === */

/* === Implementations === */

impl Direction {
    const fn step_delta(self) -> i32 {
        match self {
            Self::Positive => 1,
            Self::Negative => -1,
        }
    }
}

impl Plan {
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    #[must_use]
    pub const fn total_steps(&self) -> u32 {
        self.total_steps
    }

    #[must_use]
    pub const fn target_microdegrees(&self) -> i32 {
        self.target_microdegrees
    }

    #[must_use]
    pub fn next_delay_microseconds(&self) -> Option<u64> {
        if self.completed_steps >= self.total_steps {
            return None;
        }
        let distance_from_start = self.completed_steps.saturating_add(1);
        let distance_to_end = self.total_steps.saturating_sub(self.completed_steps);
        let ramp_distance = distance_from_start.min(distance_to_end);
        let ramp_velocity = integer_sqrt(
            2_u64.saturating_mul(u64::from(self.acceleration_steps_per_second_squared)).saturating_mul(u64::from(ramp_distance)),
        );
        let velocity = ramp_velocity.min(u64::from(self.maximum_steps_per_second)).max(1);
        Some(1_000_000 / velocity)
    }

    fn record_step(&mut self) {
        self.completed_steps = self.completed_steps.saturating_add(1);
    }

    const fn is_complete(&self) -> bool {
        self.completed_steps >= self.total_steps
    }
}

impl Axis {
    #[must_use]
    pub const fn new(steps_per_revolution: u32) -> Self {
        Self { steps_per_revolution, position_steps: 0 }
    }

    pub fn set_position(&mut self, microdegrees: i32) {
        self.position_steps = self.steps_for(microdegrees);
    }

    #[must_use]
    pub fn position_microdegrees(&self) -> i32 {
        let microdegrees =
            i64::from(self.position_steps).saturating_mul(MICRODEGREES_PER_REVOLUTION) / i64::from(self.steps_per_revolution.max(1));
        i32::try_from(microdegrees).unwrap_or(if microdegrees.is_negative() { i32::MIN } else { i32::MAX })
    }

    #[must_use]
    pub fn plan_to(
        &self, target_microdegrees: i32, maximum_microdegrees_per_second: u32, acceleration_microdegrees_per_second_squared: u32,
    ) -> Plan {
        let target_steps = self.steps_for(target_microdegrees);
        let delta = target_steps.saturating_sub(self.position_steps);
        Plan {
            direction: if delta < 0 { Direction::Negative } else { Direction::Positive },
            total_steps: delta.unsigned_abs(),
            completed_steps: 0,
            target_steps,
            target_microdegrees,
            maximum_steps_per_second: self.rate_in_steps(maximum_microdegrees_per_second).max(1),
            acceleration_steps_per_second_squared: self.rate_in_steps(acceleration_microdegrees_per_second_squared).max(1),
        }
    }

    pub fn record_step(&mut self, plan: &mut Plan) {
        self.position_steps = self.position_steps.saturating_add(plan.direction.step_delta());
        plan.record_step();
        if plan.is_complete() {
            self.position_steps = plan.target_steps;
        }
    }

    fn steps_for(self, microdegrees: i32) -> i32 {
        let numerator = i64::from(microdegrees).saturating_mul(i64::from(self.steps_per_revolution));
        let rounded = if numerator >= 0 {
            numerator.saturating_add(MICRODEGREES_PER_REVOLUTION / 2)
        } else {
            numerator.saturating_sub(MICRODEGREES_PER_REVOLUTION / 2)
        };
        let steps = rounded / MICRODEGREES_PER_REVOLUTION;
        i32::try_from(steps).unwrap_or(if steps.is_negative() { i32::MIN } else { i32::MAX })
    }

    fn rate_in_steps(self, microdegrees_per_second: u32) -> u32 {
        let steps = u64::from(microdegrees_per_second).saturating_mul(u64::from(self.steps_per_revolution))
            / u64::try_from(MICRODEGREES_PER_REVOLUTION).unwrap_or(u64::MAX);
        u32::try_from(steps).unwrap_or(u32::MAX)
    }
}

/* === Public Functions === */

#[must_use]
pub fn normalize_heading(microdegrees: i32) -> i32 {
    microdegrees.rem_euclid(FULL_TURN_MICRODEGREES)
}

#[must_use]
pub fn nearest_heading_target(current: i32, requested: i32) -> i32 {
    let current = normalize_heading(current);
    let requested = normalize_heading(requested);
    let mut delta = (requested - current).rem_euclid(FULL_TURN_MICRODEGREES);
    if delta > HALF_TURN_MICRODEGREES {
        delta -= FULL_TURN_MICRODEGREES;
    }
    current + delta
}

/* === Private Functions === */

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut low = 1;
    let mut high = value.min(u64::from(u32::MAX));
    while low <= high {
        let mid = low + (high - low) / 2;
        if mid <= value / mid {
            low = mid.saturating_add(1);
        } else {
            high = mid.saturating_sub(1);
        }
    }
    high
}

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::{Axis, Direction, nearest_heading_target, normalize_heading};

    #[test]
    fn normalizes_circular_headings() {
        assert_eq!(normalize_heading(720_000_000), 0);
        assert_eq!(normalize_heading(-90_000_000), 270_000_000);
    }

    #[test]
    fn resolves_absolute_path() {
        assert_eq!(nearest_heading_target(350_000_000, 10_000_000), 370_000_000);
        assert_eq!(nearest_heading_target(10_000_000, 350_000_000), -10_000_000);
        assert_eq!(nearest_heading_target(0, 180_000_000), 180_000_000);
    }

    #[test]
    fn plans_quarter_motor() {
        let axis = Axis::new(200);
        let positive = axis.plan_to(90_000_000, 90_000_000, 360_000_000);
        let negative = axis.plan_to(-90_000_000, 90_000_000, 360_000_000);
        assert_eq!(positive.total_steps(), 50);
        assert_eq!(positive.direction(), Direction::Positive);
        assert_eq!(negative.total_steps(), 50);
        assert_eq!(negative.direction(), Direction::Negative);
    }

    #[test]
    fn records_steps_drift() {
        let mut axis = Axis::new(200);
        let mut plan = axis.plan_to(45_000_000, 90_000_000, 360_000_000);
        while plan.next_delay_microseconds().is_some() {
            axis.record_step(&mut plan);
        }
        assert_eq!(axis.position_microdegrees(), 45_000_000);
    }

    #[test]
    fn plans_multiple_target() {
        let axis = Axis::new(200);
        let plan = axis.plan_to(720_000_000, 360_000_000, 360_000_000);
        assert_eq!(plan.total_steps(), 400);
    }

    #[test]
    fn profile_accelerates_decelerates() {
        let axis = Axis::new(200);
        let mut plan = axis.plan_to(90_000_000, 90_000_000, 360_000_000);
        let first = plan.next_delay_microseconds().unwrap();
        for _ in 0..24 {
            let mut copy = axis;
            copy.record_step(&mut plan);
        }
        let middle = plan.next_delay_microseconds().unwrap();
        for _ in 0..24 {
            let mut copy = axis;
            copy.record_step(&mut plan);
        }
        let last = plan.next_delay_microseconds().unwrap();
        assert!(middle < first);
        assert!(last >= middle);
    }
}
