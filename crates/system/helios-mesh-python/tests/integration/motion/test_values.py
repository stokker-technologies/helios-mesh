# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Motion value types and the flat StepperConfig, plus the demoted wire
escape hatch (helios_mesh.motion.wire)."""

# === Imports === #

# Standard Library Imports
# Third Party Imports
import pytest

# Local Imports
from helios_mesh.motion import Delta, Kind, StepperConfig, Target
from helios_mesh.motion.wire import (
    Configuration,
    Limits,
    Mechanics,
    Microsteps,
    Power,
    Settings,
    StepperConfiguration,
)

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def test_targets_keep_explicit() -> None:
    rotational = Target.rotational(180_000_000)
    translational = Target.translational(2500)

    assert rotational.kind == Kind.ROTATIONAL
    assert rotational.microdegrees == 180_000_000
    assert rotational.degrees == 180.0
    assert rotational.micrometres is None
    assert translational.kind == Kind.TRANSLATIONAL
    assert translational.micrometres == 2500
    assert translational.degrees is None


def test_deltas_preserve_units() -> None:
    delta = Delta.rotational(-90_000_000)

    assert delta.kind == Kind.ROTATIONAL
    assert delta.degrees == -90.0
    assert delta.micrometres is None


def test_stepper_config_only() -> None:
    stepper = StepperConfig(
        steps=200,
        microsteps=8,
        max_speed=180,
        max_acceleration=360,
        moving_pwm=50,
        holding_pwm=10,
    )

    assert stepper.steps == 200
    assert stepper.microsteps == 8
    assert stepper.reversed is False
    assert stepper.max_speed == 180
    assert stepper.max_acceleration == 360
    assert stepper.moving_pwm == 50
    assert stepper.holding_pwm == 10


def test_stepper_config_kwarg() -> None:
    stepper = StepperConfig(
        steps=200,
        microsteps=4,
        max_speed=60,
        max_acceleration=180,
        moving_pwm=25,
        holding_pwm=10,
        reversed=True,
    )

    assert stepper.reversed is True


def test_settings_keep_open() -> None:
    assert Settings().speed is None
    assert Settings().acceleration is None
    assert Settings(speed=90, acceleration=180).speed == 90


def test_composes_bounded_steppers() -> None:
    stepper = StepperConfiguration(
        Mechanics(200, Microsteps.EIGHT),
        Limits(speed=60, acceleration=180),
        Power(25, 10),
    )

    configuration = Configuration([stepper, stepper])

    assert len(configuration.steppers) == 2
    assert configuration.steppers[0].limits.speed == 60
    assert configuration.steppers[1].mechanics.microsteps.count == 8
    with pytest.raises(ValueError, match="at most four"):
        _ = Configuration([stepper] * 5)


# === Private Functions === #

# === Entry Point === #
