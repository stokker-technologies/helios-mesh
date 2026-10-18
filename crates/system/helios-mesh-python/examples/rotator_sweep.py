# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""Configure, home, and sweep both motion channels of one node."""

# === Imports === #

# Standard Library Imports
from argparse import ArgumentParser, Namespace
from asyncio import (
    AbstractEventLoop,
    Event,
    Future,
    ensure_future,
    gather,
    get_running_loop,
    run,
    sleep,
)
from collections.abc import Awaitable
from datetime import datetime
from signal import SIGINT, signal
from types import FrameType
from typing import TypeVar

# Third Party Imports
# Local Imports
from helios_mesh import (
    BusyError,
    DeviceSelector,
    GatewayNode,
    NodeType,
    NodeUnreachableError,
    OperationStoppedError,
    RotatorNode,
    UnavailableError,
)
from helios_mesh.motion import Axis, StepperConfig
from helios_mesh.node import NodeId, Operation, OperationSnapshot

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

_T = TypeVar("_T")

# === Classes === #

# === Public Functions === #


async def gather_or_cancel(*awaitables: Awaitable[_T]) -> list[_T]:
    """Run every awaitable concurrently; on any failure, cancel and await
    every other one before propagating.

    Plain `gather()` raises as soon as one awaitable fails but leaves its
    siblings running unobserved in the background -- here, that could mean
    a `submit_with_retry` or `Operation.completed()` for one channel keeps
    running (and could still submit a move) after its sweep has already
    been treated as failed.

    If this coroutine is itself cancelled from outside while awaiting the
    cleanup `gather(..., return_exceptions=True)`, that second wait can be
    interrupted too, leaving a task unawaited; acceptable for a
    demonstration script, not a guarantee a library would want to make.
    """
    tasks: list[Future[_T]] = [ensure_future(awaitable) for awaitable in awaitables]
    try:
        return await gather(*tasks)
    finally:
        for task in tasks:
            if not task.done():
                _ = task.cancel()
        _ = await gather(*tasks, return_exceptions=True)


def parse_args() -> Namespace:
    parser = ArgumentParser(description=__doc__)
    _ = parser.add_argument("--node", type=int, help="stable motion node ID; defaults to the attached node")
    _ = parser.add_argument("--device", type=int, help="stable USB node ID of the attachment point")
    _ = parser.add_argument("--port", help="explicit gateway serial port")
    _ = parser.add_argument("--angle", type=float, default=360.0, help="sweep angle in degrees")
    _ = parser.add_argument("--speed", type=int, help="optional speed in degrees per second")
    _ = parser.add_argument(
        "--acceleration",
        type=int,
        help="optional acceleration in degrees per second squared",
    )
    _ = parser.add_argument("--dwell", type=float, default=1.0, help="seconds between moves")
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
    """
    try:
        loop.add_signal_handler(SIGINT, stop.set)
    except NotImplementedError:

        def request_stop(signum: int, frame: FrameType | None) -> None:
            del signum, frame
            _ = loop.call_soon_threadsafe(stop.set)

        _ = signal(SIGINT, request_stop)


async def main() -> None:
    args = parse_args()
    stop = Event()
    stop_on_sigint(get_running_loop(), stop)

    node = await resolve_node(args)
    stepper = StepperConfig(
        steps=200,
        microsteps=4,
        max_speed=180,
        max_acceleration=360,
        moving_pwm=50,
        holding_pwm=10,
    )
    await node.configure(stepper, stepper)

    speed: int | None = args.speed
    acceleration: int | None = args.acceleration
    try:
        await run_cycle(node, speed, acceleration, stop, args)
    except NodeUnreachableError as error:
        # Unlike a stop, a confirmed-dark node is not a recoverable cycle
        # event: the gateway's own registry says so, so retrying the loop
        # would just repeat the same field-defect hang this error exists to
        # short-circuit. Exit cleanly instead of a raw traceback.
        print(f"{error} -- check power/RF, then power-cycle if needed")


async def run_cycle(
    node: RotatorNode,
    speed: int | None,
    acceleration: int | None,
    stop: Event,
    args: Namespace,
) -> None:
    """Select both axes, home, and sweep until the operator stops the run.

    An unexpected stop is a recoverable cycle event: re-home the same selected
    axes and resume rather than rebuilding the bundle or crashing the demo.
    """
    guard = await node.axes()
    first, second = guard.axes
    homing = await gather_or_cancel(first.start_zero(), second.start_zero())
    _ = await gather_or_cancel(*(operation.completed() for operation in homing))
    while not stop.is_set():
        for direction in (1.0, -1.0):
            started = datetime.now()
            try:
                results = await sweep_once(first, second, direction * args.angle, speed, acceleration)
            except OperationStoppedError as error:
                print(f"[{started:%H:%M:%S.%f}] stopped: {error}; re-homing the selected axes")
                if stop.is_set():
                    return
                homing = await gather_or_cancel(first.start_zero(), second.start_zero())
                _ = await gather_or_cancel(*(operation.completed() for operation in homing))
                break
            finished = datetime.now()
            elapsed = (finished - started).total_seconds()
            states = ", ".join(f"channel {snapshot.channel.value}={snapshot.state}" for snapshot in results)
            print(f"[{started:%H:%M:%S.%f} -> {finished:%H:%M:%S.%f}, {elapsed:.3f}s] completed: {states}")
            await sleep(args.dwell)


async def sweep_once(first: Axis, second: Axis, angle: float, speed: int | None, acceleration: int | None) -> list[OperationSnapshot]:
    """Run one opposed relative move, retrying each axis independently.

    Each axis's admission is retried on its own, not the whole opposed
    pair: retrying the pair together would needlessly resubmit a move on
    whichever axis already got its request through, double-applying a
    delta that never needed retrying just because its sibling did.

    ``UnavailableError`` marks a retryable exchange failure on the
    half-duplex radio. ``BusyError`` marks a collision with an earlier
    admission on the same axis that this host lost track of -- its
    response never arrived, but the move it started still ran on the node.
    Retrying that axis's admission can still double-apply in that case,
    since the lost operation identifier leaves no way to await the
    original move instead of resubmitting; this demonstration accepts
    that residual risk, now scoped to the one axis it can happen on.
    """
    operations = await gather_or_cancel(
        submit_with_retry(first, angle, speed, acceleration),
        submit_with_retry(second, -angle, speed, acceleration),
    )
    return await gather_or_cancel(*(operation.completed() for operation in operations))


async def submit_with_retry(axis: Axis, degrees: float, speed: int | None, acceleration: int | None) -> Operation:
    """Admit one axis's relative turn, retrying only this axis.

    A collision on this axis's own busy motor (``BusyError``) is retried
    exactly like a transient mesh loss (``UnavailableError``): both mean
    this attempt did not visibly succeed, so trying again is the only
    option available to the caller.
    """
    for attempt in range(3):
        try:
            return await axis.start_turn(degrees, speed=speed, acceleration=acceleration)
        except (UnavailableError, BusyError) as error:
            print(f"{type(error).__name__} on channel {axis.id.value}; retrying admission (attempt {attempt + 2}/3)")
            await sleep(1.0)
    raise UnavailableError(f"channel {axis.id.value} unavailable across repeated admission attempts")


# === Private Functions === #

# === Entry Point === #

if __name__ == "__main__":
    run(main())
