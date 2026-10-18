# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Sweep motor tuning profiles and recommend a zero-loss-fidelity default.

Open-loop steppers physically lose steps when commanded timing outruns the
hardware. This script drives one motion channel through a grid of tuning
profiles (microsteps x speed x acceleration x moving-power), harvests the
firmware's per-move step-scheduling and I2C-bus timing telemetry
(``helios_mesh.motion.Timing``) from the node's journal, and scores every profile
against a zero-loss criterion: no delayed steps, bounded step lateness, and a
bounded worst-case PCA9685 I2C commit time.

The grid runs over microsteps values 4 and 8 only. The Adafruit Motor
FeatherWing's stepper driver (PCA9685-driven, via ``helios_mesh.motion.StepperConfig
.microsteps``) supports exactly those two microstepping modes -- there is no
1x or 2x microstepping to sweep, whatever a naive "microsteps 1/2/4/8" grid
description might suggest.

Electronic fidelity is necessary but not sufficient for zero *mechanical*
step loss -- see the physical-verification protocol printed at the end of
every run.
"""

# === Imports === #

# Standard Library Imports
from argparse import ArgumentParser, Namespace
from asyncio import AbstractEventLoop, Event, get_running_loop, run, sleep
from dataclasses import dataclass, field
from signal import SIGINT, signal
from types import FrameType

# Third Party Imports
# Local Imports
from helios_mesh import (
    BusyError,
    DeviceSelector,
    GatewayNode,
    NodeType,
    OperationFailedError,
    OperationStoppedError,
    RotatorNode,
    UnavailableError,
)
from helios_mesh.journal import Cursor
from helios_mesh.motion import Axis, StepperConfig, Timing
from helios_mesh.node import NodeId, Operation

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# The two microstepping modes the Motor FeatherWing's stepper driver actually
# supports -- see the module docstring for why there is no 1x/2x to add here.
# Plain ints, not the demoted `helios_mesh.motion.wire.Microsteps` enum: the flat
# `StepperConfig.microsteps` this script builds takes a bare int (4 or 8).
_MICROSTEPS_GRID: tuple[int, ...] = (4, 8)

# A companion firmware investigation established that one single-port step's
# I2C commit is a hardware-forced 2-3 separate small transactions (register
# layout, plus a documented PWM-before-direction safety ordering, prevent
# merging them), with a theoretical worst case around 200us of wire+register
# overhead at the bus's already-maximum 1MHz clock. 500us leaves headroom for
# real overhead while still catching genuine regressions or bus contention.
_DEFAULT_I2C_BUDGET_US = 500
_DEFAULT_LATENESS_FRACTION = 0.25

# === Types === #

# === Classes === #


@dataclass(frozen=True)
class Profile:
    """One point in the tuning grid."""

    microsteps: int
    speed: int
    acceleration: int
    moving_power: int

    def __str__(self) -> str:
        return (
            f"microsteps={self.microsteps}x speed={self.speed}deg/s "
            f"acceleration={self.acceleration}deg/s^2 moving_power={self.moving_power}%"
        )


@dataclass
class Stats:
    """Aggregated confirmation-move telemetry for one profile."""

    moves_recorded: int = 0
    harness_errors: int = 0
    total_steps: int = 0
    delayed_steps_sum: int = 0
    max_lateness_us: int = 0
    worst_i2c_update_us: int = 0


@dataclass(frozen=True)
class Verdict:
    passed: bool
    failed_gates: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class Result:
    profile: Profile
    stats: Stats
    step_period_us: float
    verdict: Verdict


# === Public Functions === #


def accumulate(stats: Stats, timing: Timing | None) -> None:
    """Fold one move's telemetry into a profile's running stats.

    ``timing is None`` means `next_timing` already printed a harness-error
    warning for this move; count it and move on rather than guessing numbers
    for a move the harness could not observe.
    """
    if timing is None:
        stats.harness_errors += 1
        return
    stats.moves_recorded += 1
    stats.total_steps += timing.steps
    stats.delayed_steps_sum += timing.delayed_steps
    stats.max_lateness_us = max(stats.max_lateness_us, timing.maximum_lateness_us)
    stats.worst_i2c_update_us = max(stats.worst_i2c_update_us, timing.maximum_i2c_update_us)


# timing.maximum_output_queue_us is intentionally never read here: it is

# a structurally near-zero/unreliable measurement in the firmware today

# and must not gate any pass/fail decision.


def step_period_us(full_steps: int, microsteps: int, speed: int) -> float:
    """Time between microsteps for one profile.

    Sanity check (the field evidence's worked example): 200 full steps x 4
    microsteps x 180 deg/s -> 400 steps/s -> 2500us/step. This formula
    reproduces that exactly: 1_000_000 * 360 / (200 * 4 * 180) == 2500.0.
    """
    return 1_000_000 * 360 / (full_steps * microsteps * speed)


def evaluate_zero_loss(
    stats: Stats,
    period_us: float,
    *,
    lateness_fraction: float,
    i2c_budget_us: int,
) -> Verdict:
    """The zero-loss pass/fail criterion, and only that criterion.

    Three independent gates over one profile's aggregated confirmation-move
    stats. Deliberately blind to *why* the stats look the way they do --
    harness bookkeeping (e.g. missing telemetry) is `evaluate_profile`'s
    concern, not this function's.

    Using the aggregated max/sum across a profile's moves is equivalent to
    checking every individual move: a sum of non-negative delayed-step
    counts is zero iff every move's count was zero, and a max is under a
    threshold iff every move's value was under it.
    """
    failed: list[str] = []
    if stats.delayed_steps_sum > 0:
        failed.append(f"delayed_steps (sum={stats.delayed_steps_sum} across {stats.moves_recorded} recorded move(s), want 0)")
    lateness_budget_us = lateness_fraction * period_us
    if stats.max_lateness_us > lateness_budget_us:
        failed.append(
            f"max_lateness ({stats.max_lateness_us}us > {lateness_budget_us:.0f}us = {lateness_fraction:g} x {period_us:.0f}us step period)"
        )
    if stats.worst_i2c_update_us > i2c_budget_us:
        failed.append(f"i2c_budget ({stats.worst_i2c_update_us}us > {i2c_budget_us}us)")
    return Verdict(passed=not failed, failed_gates=failed)


def evaluate_profile(
    stats: Stats,
    period_us: float,
    *,
    lateness_fraction: float,
    i2c_budget_us: int,
) -> Verdict:
    """The zero-loss criterion, plus the harness's own integrity gate.

    A profile with any move whose telemetry could not be unambiguously
    attributed (`Stats.harness_errors`) never earns a PASS: the harness
    could not observe that move, so it cannot vouch for it, regardless of
    what the moves it did observe looked like.
    """
    verdict = evaluate_zero_loss(stats, period_us, lateness_fraction=lateness_fraction, i2c_budget_us=i2c_budget_us)
    failed = list(verdict.failed_gates)
    if stats.harness_errors:
        failed.append(f"telemetry (harness could not read exactly one Timing for {stats.harness_errors} move(s))")
    return Verdict(passed=not failed, failed_gates=failed)


def failure_margin(stats: Stats, period_us: float, *, lateness_fraction: float, i2c_budget_us: int) -> float:
    """A rough, unit-normalized 'how far from passing' score, smaller is closer.

    Not part of the zero-loss criterion -- only used to rank FAILing profiles
    against each other when recommending a manual-tuning starting point.
    Each numeric gate contributes its overshoot as a fraction of its own
    budget so gates with different units are comparable; a harness telemetry
    gap counts as a full gate failure on its own.
    """
    lateness_budget_us = max(lateness_fraction * period_us, 1.0)
    lateness_overshoot = max(0.0, stats.max_lateness_us - lateness_budget_us) / lateness_budget_us
    i2c_overshoot = max(0.0, stats.worst_i2c_update_us - i2c_budget_us) / max(i2c_budget_us, 1)
    delayed_overshoot = stats.delayed_steps_sum / max(stats.moves_recorded, 1)
    return lateness_overshoot + i2c_overshoot + delayed_overshoot + stats.harness_errors


def int_list(raw: str) -> list[int]:
    return [int(item.strip()) for item in raw.split(",") if item.strip()]


def parse_args() -> Namespace:
    parser = ArgumentParser(description=__doc__)
    _ = parser.add_argument("--node", type=int, help="stable motion node ID; defaults to the attached node")
    _ = parser.add_argument("--device", type=int, help="stable USB node ID of the attachment point")
    _ = parser.add_argument("--port", help="explicit gateway serial port")
    _ = parser.add_argument(
        "--channel",
        type=int,
        default=0,
        help="advertised motion channel index to drive (default: 0)",
    )
    _ = parser.add_argument("--full-steps", type=int, default=200, help="motor full steps per revolution (default: 200)")
    _ = parser.add_argument("--angle", type=float, default=10.0, help="degrees per confirmation swing (default: 10.0)")
    _ = parser.add_argument(
        "--cycles",
        type=int,
        default=3,
        help="back-and-forth move pairs per profile for scoring (default: 3)",
    )
    _ = parser.add_argument(
        "--speeds",
        type=int_list,
        default=[90, 180, 270],
        help="comma-separated deg/s grid values (default: 90,180,270)",
    )
    _ = parser.add_argument(
        "--accelerations",
        type=int_list,
        default=[360, 720],
        help="comma-separated deg/s^2 grid values (default: 360,720)",
    )
    _ = parser.add_argument(
        "--moving-power",
        type=int_list,
        default=[80],
        help="comma-separated 0-100 moving-power grid values (default: 80)",
    )
    _ = parser.add_argument(
        "--holding-power",
        type=int,
        default=10,
        help="0-100 holding power, held constant across the grid (default: 10)",
    )
    _ = parser.add_argument(
        "--lateness-fraction",
        type=float,
        default=_DEFAULT_LATENESS_FRACTION,
        help="max allowed step lateness as a fraction of the step period (default: 0.25)",
    )
    _ = parser.add_argument(
        "--i2c-budget-us",
        type=int,
        default=_DEFAULT_I2C_BUDGET_US,
        help="max allowed worst-case PCA9685 I2C commit time in microseconds (default: 500)",
    )
    _ = parser.add_argument(
        "--confirm",
        action="store_true",
        help="after the sweep, physically verify the recommended (or pinned) profile",
    )
    _ = parser.add_argument(
        "--confirm-cycles",
        type=int,
        default=10,
        help="full back-and-forth round trips to run during --confirm (default: 10)",
    )
    # Named --confirm-<field>, not --speed/--acceleration/--moving-power,
    # deliberately: those names differ from the sweep's --speeds/
    # --accelerations/--moving-power by only a letter, and mixing them up
    # would silently skip the whole sweep instead of running it.
    _ = parser.add_argument(
        "--confirm-microsteps",
        type=int,
        choices=(4, 8),
        help="pin confirmation to this microsteps value instead of the sweep's "
        "recommendation (requires the other three --confirm-* pins too)",
    )
    _ = parser.add_argument(
        "--confirm-speed",
        type=int,
        help="pin confirmation to this speed in deg/s (requires the other three --confirm-* pins too)",
    )
    _ = parser.add_argument(
        "--confirm-acceleration",
        type=int,
        help="pin confirmation to this acceleration in deg/s^2 (requires the other three --confirm-* pins too)",
    )
    _ = parser.add_argument(
        "--confirm-moving-power",
        type=int,
        help="pin confirmation to this 0-100 moving power (requires the other three --confirm-* pins too)",
    )
    return parser.parse_args()


def selector(args: Namespace) -> DeviceSelector:
    if args.device is not None:
        return DeviceSelector.id(NodeId(args.device))
    if args.port is not None:
        return DeviceSelector.port(args.port)
    return DeviceSelector.only()


async def resolve_node(args: Namespace) -> RotatorNode:
    """Select a directly attached rotator or a typed mesh roster entry."""
    if args.node is None and (args.device is not None or args.port is not None):
        return await RotatorNode.connect(selector(args))

    gateway = await GatewayNode.connect(selector(args))
    target = NodeId(args.node) if args.node is not None else await gateway.scan_one(NodeType.ROTATOR)
    node = await gateway.connect_node(target)
    if not isinstance(node, RotatorNode):
        raise ValueError("the selected node is not a rotator")
    return node


def stop_on_sigint(loop: AbstractEventLoop, stop: Event) -> None:
    """Request a graceful stop on Ctrl-C, on every platform.

    ``add_signal_handler`` is Unix-only; Windows event loops raise
    ``NotImplementedError``, so fall back to a classic signal handler that
    trampolines into the loop.

    Note: while a ``--confirm`` round trip is paused at its
    ``input()`` prompt, that call blocks the interpreter outright, so a
    Ctrl-C there is only noticed once Enter is pressed and control returns
    to the event loop.
    """
    try:
        loop.add_signal_handler(SIGINT, stop.set)
    except NotImplementedError:

        def request_stop(signum: int, frame: FrameType | None) -> None:
            del signum, frame
            _ = loop.call_soon_threadsafe(stop.set)

        _ = signal(SIGINT, request_stop)


async def submit_with_retry(axis: Axis, angle: float, speed: int, acceleration: int) -> Operation:
    """Admit one relative move, retrying only on a transient failure.

    A collision on this axis's own busy motor (``BusyError``) is
    retried exactly like a transient mesh loss (``UnavailableError``): both
    mean this attempt did not visibly succeed, so trying again is the only
    option available to the caller.
    """
    for attempt in range(3):
        try:
            return await axis.start_turn(angle, speed=speed, acceleration=acceleration)
        except (UnavailableError, BusyError) as error:
            print(f"  {type(error).__name__} on channel {axis.id.value}; retrying admission (attempt {attempt + 2}/3)")
            await sleep(1.0)
    raise UnavailableError(f"channel {axis.id.value} unavailable across repeated admission attempts")


async def next_timing(cursor: Cursor, node: RotatorNode) -> Timing | None:
    """Timing for the one journal entry expected after a just-completed move.

    In this single-channel sequential harness, exactly one newly observed
    journal entry should wrap a completed motion event. Zero or more than
    one is a harness-level problem -- print a clear warning and let the
    caller count the move as failed rather than silently guessing which
    entry to use.
    """
    entries = await cursor.next(node.node)
    timings = [entry.event.timing for entry in entries if entry.event.timing is not None]
    if len(timings) != 1:
        print(
            f"  warning: expected exactly one timing-bearing journal entry after this "
            f"move's completion, found {len(timings)}; treating as a harness error"
        )
        return None
    return timings[0]


async def run_move(axis: Axis, node: RotatorNode, cursor: Cursor, angle: float, speed: int, acceleration: int) -> Timing | None:
    try:
        operation = await submit_with_retry(axis, angle, speed, acceleration)
        _ = await operation.completed()
    except (OperationStoppedError, OperationFailedError) as error:
        print(f"  warning: move did not complete cleanly ({type(error).__name__}: {error}); counting as a harness error for this move")
        return None
    return await next_timing(cursor, node)


async def run_profile(axis: Axis, node: RotatorNode, cursor: Cursor, profile: Profile, args: Namespace, stop: Event) -> Stats:
    stats = Stats()
    for cycle in range(args.cycles):
        if stop.is_set():
            print(f"  stop requested; ending after {cycle}/{args.cycles} cycle(s)")
            break
        for direction in (1.0, -1.0):
            timing = await run_move(axis, node, cursor, direction * args.angle, profile.speed, profile.acceleration)
            accumulate(stats, timing)
    return stats


def build_grid(args: Namespace) -> list[Profile]:
    """Build the sweep grid in a stable, reconfiguration-minimizing order.

    Nesting: microsteps (outer) -> speed -> acceleration -> moving power
    (inner). The active stepper config's ``max_speed``/``max_acceleration``
    ceiling (see ``configuration_ceiling``/``stepper_configuration``) is a
    per-microsteps constant, so speed and acceleration vary through
    per-operation settings without redundant reconfiguration. Moving power
    remains part of the boot-local channel definition.

    Default grid size, with the documented default lists: 2 microsteps x
    3 speeds x 2 accelerations x 1 moving power = 12 profiles -- inside the
    "modest ~12-20 profiles" range this harness is meant to stay in.
    """
    return [
        Profile(microsteps=microsteps, speed=speed, acceleration=acceleration, moving_power=power)
        for microsteps in _MICROSTEPS_GRID
        for speed in args.speeds
        for acceleration in args.accelerations
        for power in args.moving_power
    ]


def pinned_profile(args: Namespace) -> Profile | None:
    pins = (
        args.confirm_microsteps,
        args.confirm_speed,
        args.confirm_acceleration,
        args.confirm_moving_power,
    )
    if all(pin is None for pin in pins):
        return None
    if any(pin is None for pin in pins):
        raise SystemExit(
            "--confirm-microsteps, --confirm-speed, --confirm-acceleration, and "
            "--confirm-moving-power must all be given together to pin a single "
            "confirmation profile, or none of them to run the sweep"
        )
    return Profile(
        microsteps=args.confirm_microsteps,
        speed=args.confirm_speed,
        acceleration=args.confirm_acceleration,
        moving_power=args.confirm_moving_power,
    )


def configuration_ceiling(args: Namespace, pinned: Profile | None) -> tuple[int, int]:
    """The boot-local speed/acceleration ceiling to configure.

    For the sweep, this is the grid's own maximum -- wide enough to hold
    every profile's per-move speed/acceleration within it, configured once
    per microsteps value rather than once per grid point. For a pinned
    single profile, there is no grid to bound, so the ceiling is exactly
    that profile's own speed/acceleration.
    """
    if pinned is not None:
        return pinned.speed, pinned.acceleration
    return max(args.speeds), max(args.accelerations)


def stepper_configuration(args: Namespace, profile: Profile, *, ceiling_speed: int, ceiling_acceleration: int) -> StepperConfig:
    return StepperConfig(
        steps=args.full_steps,
        microsteps=profile.microsteps,
        max_speed=ceiling_speed,
        max_acceleration=ceiling_acceleration,
        moving_pwm=profile.moving_power,
        holding_pwm=args.holding_power,
    )


async def configure_if_changed(node: RotatorNode, config: StepperConfig, last: StepperConfig | None) -> StepperConfig:
    """Apply ``config`` to every channel iff it differs from ``last``.

    ``node.configure()`` atomically replaces *every* advertised channel's
    in-memory definition for the current boot -- there is no per-channel
    write and no way to read back a channel's current configuration. This
    harness only ever moves the one channel named by --channel, but every
    configure() call here also changes the mechanics, limits, and power of
    every *other* advertised channel until the next configure or restart.
    That is a real, operator-visible side effect on a shared node; it is
    unavoidable given the API, but worth knowing before running this against
    a node that other channels depend on.
    """
    if last is not None and config == last:
        return last
    channel_count = len(await node.channels())
    await node.configure(*([config] * channel_count))
    return config


def format_verdict(verdict: Verdict) -> str:
    if verdict.passed:
        return "PASS"
    return "FAIL: " + "; ".join(verdict.failed_gates)


def print_table(results: list[Result]) -> None:
    headers = [
        "microsteps",
        "speed(d/s)",
        "accel(d/s^2)",
        "power(%)",
        "period(us)",
        "steps",
        "delayed",
        "max_late(us)",
        "worst_i2c(us)",
        "verdict",
    ]
    print()
    print("=== sweep results ===")
    if not results:
        print("(no profiles were run)")
        return
    rows = [
        [
            str(result.profile.microsteps),
            str(result.profile.speed),
            str(result.profile.acceleration),
            str(result.profile.moving_power),
            f"{result.step_period_us:.0f}",
            str(result.stats.total_steps),
            str(result.stats.delayed_steps_sum),
            str(result.stats.max_lateness_us),
            str(result.stats.worst_i2c_update_us),
            format_verdict(result.verdict),
        ]
        for result in results
    ]
    widths = [max(len(headers[column]), *(len(row[column]) for row in rows)) for column in range(len(headers))]

    def line(cells: list[str]) -> str:
        return "  ".join(cell.ljust(width) for cell, width in zip(cells, widths, strict=True))

    print(line(headers))
    print(line(["-" * width for width in widths]))
    for row in rows:
        print(line(row))


def recommend(results: list[Result]) -> Result | None:
    passing = [result for result in results if result.verdict.passed]
    if not passing:
        return None
    return min(
        passing,
        key=lambda result: (
            -result.profile.speed,
            result.stats.max_lateness_us,
            result.stats.worst_i2c_update_us,
        ),
    )


def closest_failing(results: list[Result], args: Namespace) -> Result:
    return min(
        results,
        key=lambda result: (
            len(result.verdict.failed_gates),
            failure_margin(
                result.stats,
                result.step_period_us,
                lateness_fraction=args.lateness_fraction,
                i2c_budget_us=args.i2c_budget_us,
            ),
        ),
    )


def print_recommendation(chosen: Result, args: Namespace, *, ceiling_speed: int, ceiling_acceleration: int) -> None:
    print()
    print("=== recommendation ===")
    print(f"recommended default: {chosen.profile}")
    print(f"  max lateness observed: {chosen.stats.max_lateness_us}us (budget {args.lateness_fraction * chosen.step_period_us:.0f}us)")
    print(f"  worst I2C update observed: {chosen.stats.worst_i2c_update_us}us (budget {args.i2c_budget_us}us)")
    print("  use:")
    # ceiling_speed/ceiling_acceleration are the boot-local configuration
    # actually validated during the sweep for this microsteps value (the
    # grid's own maximum), not necessarily equal to this profile's own
    # per-move speed/acceleration printed on the second line below.
    print(
        f"    StepperConfig(steps={args.full_steps}, microsteps={chosen.profile.microsteps}, "
        f"max_speed={ceiling_speed}, max_acceleration={ceiling_acceleration}, "
        f"moving_pwm={chosen.profile.moving_power}, holding_pwm={args.holding_power})"
    )
    print(f"    axis.turn(degrees=..., speed={chosen.profile.speed}, acceleration={chosen.profile.acceleration})")


def print_no_recommendation(fallback: Result) -> None:
    print()
    print("=== recommendation ===")
    print("no profile passed the zero-loss criterion.")
    print(f"closest failing profile (starting point for manual tuning): {fallback.profile}")
    print(f"  failed: {'; '.join(fallback.verdict.failed_gates)}")


async def run_confirmation(axis: Axis, node: RotatorNode, cursor: Cursor, profile: Profile, args: Namespace, stop: Event) -> None:
    """Physically verify one profile with real, observed round trips.

    Each round trip is a full +angle/-angle pair that returns the rotor to
    its starting position; the operator eyeballs the mark against the fixed
    reference between rounds. Electronic anomalies (missing telemetry, or
    any non-zero delayed_steps) are counted and reported, but the pass/fail
    call for this run is the operator's, made against the physical mark --
    that is the entire point of this protocol.
    """
    anomalies = 0
    rounds = args.confirm_cycles
    print()
    print(f"=== confirmation: {rounds} round trip(s) on {profile} ===")
    for round_index in range(1, rounds + 1):
        if stop.is_set():
            print(f"  stop requested; ending after {round_index - 1}/{rounds} round trip(s)")
            break
        for direction in (1.0, -1.0):
            timing = await run_move(axis, node, cursor, direction * args.angle, profile.speed, profile.acceleration)
            if timing is None:
                anomalies += 1
                print(f"  anomaly: no timing telemetry for this move (running total: {anomalies})")
            elif timing.delayed_steps > 0:
                anomalies += 1
                print(f"  anomaly: {timing.delayed_steps} delayed step(s) this move (running total: {anomalies})")
        print(f"  round {round_index}/{rounds} complete -- rotor should be back at the start")
        _ = input("  press Enter after checking the mark...")
    print(f"confirmation finished: {anomalies} electronic anomaly event(s) across {rounds} round trip(s)")


def print_verification_protocol(*, confirmed: bool) -> None:
    print()
    print("=== physical verification ===")
    print("Electronic fidelity (zero delayed steps, bounded step lateness, bounded PCA9685")
    print("I2C commit time) is NECESSARY but NOT SUFFICIENT for zero mechanical step loss:")
    print("torque, load, and microstep current settling can still lose real steps even")
    print("when every timing number above looks perfect.")
    print()
    print("Before trusting a profile: mark the rotor's current physical position (e.g. a")
    print("piece of tape or a marker line against a fixed reference point on the housing)")
    print("and physically count turns. That is the only real confirmation; nothing this")
    print("script measures can substitute for it.")
    if confirmed:
        print()
        print("The confirmation run above already executed -- compare the mark to the")
        print("reference now that the run is done.")


async def main() -> None:
    args = parse_args()
    stop = Event()
    stop_on_sigint(get_running_loop(), stop)

    node = await resolve_node(args)

    descriptions = await node.channels()
    channel_index: int = args.channel
    if not 0 <= channel_index < len(descriptions):
        raise SystemExit(f"--channel {channel_index} out of range: node advertises {len(descriptions)} motion channel(s)")
    channel_id = descriptions[channel_index].channel

    pinned = pinned_profile(args)
    ceiling_speed, ceiling_acceleration = configuration_ceiling(args, pinned)

    cursor = Cursor()
    # A fresh Cursor starts behind this node's entire bounded journal,
    # which on a bench node has likely already got old timing-bearing
    # entries in it. Drain everything outstanding now so the first scored
    # move's next_timing() call only
    # ever sees that move's own entry.
    while await cursor.next(node.node):
        pass

    if pinned is not None:
        print(f"pinned profile given; skipping the sweep and confirming directly: {pinned}")
        config = stepper_configuration(args, pinned, ceiling_speed=ceiling_speed, ceiling_acceleration=ceiling_acceleration)
        # Unconditional: there is no "last" configuration to compare
        # against outside a sweep, so this always writes -- unlike the
        # sweep loop below, which routes through configure_if_changed to
        # skip redundant reconfiguration across grid points.
        # Configuring resets the channel's homing state, so re-home the
        # selected axis after applying it.
        channel_count = len(descriptions)
        await node.configure(*([config] * channel_count))
        guard = await node.axes(channels=[channel_id])
        axis = guard.axes[0]
        _ = await axis.zero()
        await run_confirmation(axis, node, cursor, pinned, args, stop)
        print_verification_protocol(confirmed=True)
        return

    profiles = build_grid(args)
    results: list[Result] = []
    last_configuration: StepperConfig | None = None
    for profile in profiles:
        if stop.is_set():
            print("stop requested; ending the sweep early")
            break
        config = stepper_configuration(
            args,
            profile,
            ceiling_speed=ceiling_speed,
            ceiling_acceleration=ceiling_acceleration,
        )
        reconfigured = last_configuration != config
        last_configuration = await configure_if_changed(node, config, last_configuration)
        period = step_period_us(args.full_steps, profile.microsteps, profile.speed)
        # A reconfigure wiped the homing state, so re-home before scoring.
        guard = await node.axes(channels=[channel_id])
        axis = guard.axes[0]
        if reconfigured:
            _ = await axis.zero()
        stats = await run_profile(axis, node, cursor, profile, args, stop)
        verdict = evaluate_profile(
            stats,
            period,
            lateness_fraction=args.lateness_fraction,
            i2c_budget_us=args.i2c_budget_us,
        )
        results.append(Result(profile, stats, period, verdict))
        print(f"[{len(results)}/{len(profiles)}] {profile} -> {format_verdict(verdict)}")

    print_table(results)
    chosen = recommend(results)
    if chosen is not None:
        print_recommendation(chosen, args, ceiling_speed=ceiling_speed, ceiling_acceleration=ceiling_acceleration)
    elif results:
        print_no_recommendation(closest_failing(results, args))
    else:
        print()
        print("=== recommendation ===")
        print("no profiles were run; nothing to recommend")

    confirmed = False
    if args.confirm:
        if chosen is not None:
            # The last-run profile may not be the chosen one: configure the
            # winner, then re-home before the confirmation run.
            winner = stepper_configuration(
                args,
                chosen.profile,
                ceiling_speed=ceiling_speed,
                ceiling_acceleration=ceiling_acceleration,
            )
            channel_count = len(descriptions)
            await node.configure(*([winner] * channel_count))
            guard = await node.axes(channels=[channel_id])
            axis = guard.axes[0]
            _ = await axis.zero()
            await run_confirmation(axis, node, cursor, chosen.profile, args, stop)
            confirmed = True
        else:
            print()
            print(
                "--confirm given but no profile passed the zero-loss criterion; "
                "skipping the physical confirmation run -- fix the closest-failing "
                "profile above first"
            )
    print_verification_protocol(confirmed=confirmed)


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    run(main())
