# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

"""The `awaitable()` bridge (`crates/system/helios-mesh-python/src/runtime.rs`) yields to
the event loop and honors task cancellation.

`helios_mesh`'s own fake transport (`helios_mesh::transport::testing`) is
`#[cfg(test)] pub(crate)`, unreachable from this crate, so a pending,
never-completing binding wait cannot be faked through a real `Operation`.
`helios_mesh._native._PendingProbe` (`crates/system/helios-mesh-python/src/testing.rs`) is the
smallest seam that fakes it: an awaitable that never resolves on its own,
paired with a flag recording whether its Rust future was dropped.

Note: `asyncio.timeout()` (the context-manager form the wrapped-await
requirement names) is Python 3.11+; this project's basedpyright config
targets 3.10 for `tests`, so these use the equally-representative
`asyncio.wait_for()` wrapper form instead. Both cancel the awaited future
the same way; see the final report for this as a flagged follow-up.
"""

# === Imports === #

# Standard Library Imports
from asyncio import CancelledError, ensure_future, get_running_loop, run, sleep, wait_for
from asyncio import TimeoutError as AsyncTimeoutError

# Third Party Imports
import pytest

# Local Imports
from helios_mesh import DeviceNotFoundError, NodeHandle
from helios_mesh._native import _PendingProbe

# pyright: ignore[reportPrivateUsage]
from helios_mesh.node import DeviceSelector, NodeId

# Type Checking Imports

# === Public Exports === #

# === Constants === #

# === Types === #

# === Classes === #

# === Public Functions === #


def test_loop_keeps_flight() -> None:
    """A `call_soon` scheduled just before a real async binding call must
    run before that call's awaitable resolves, proving the interpreter
    thread was not synchronously blocked for the call's whole duration."""

    async def scenario() -> int:
        loop = get_running_loop()
        ticks = 0

        def tick() -> None:
            nonlocal ticks
            ticks += 1
            _ = loop.call_soon(tick)

        _ = loop.call_soon(tick)
        with pytest.raises(DeviceNotFoundError):
            _ = await NodeHandle.connect(DeviceSelector.id(NodeId(998877)))
        return ticks

    ticks = run(scenario())
    assert ticks > 0, "the loop never ran a scheduled callback while the binding call was pending"


def test_scheduled_callback_flight() -> None:
    """SIGINT-style responsiveness: a `call_later` callback (standing in for
    `loop.add_signal_handler`'s trampoline) must fire while a binding
    await is pending forever, not only after it eventually resolves."""

    async def scenario() -> bool:
        loop = get_running_loop()
        fired = loop.create_future()
        _ = loop.call_later(0.01, lambda: None if fired.done() else fired.set_result(True))

        probe = _PendingProbe()
        waiting = ensure_future(probe.wait())
        result = await fired
        _ = waiting.cancel()
        with pytest.raises(CancelledError):
            await waiting
        return result

    assert run(scenario()) is True


def test_wait_timeout_wait() -> None:
    """`asyncio.wait_for` around a binding call that never resolves on its
    own must still raise `TimeoutError` promptly, proving the awaitable
    genuinely yields instead of blocking the loop for the full wait."""

    async def scenario() -> None:
        probe = _PendingProbe()
        with pytest.raises(AsyncTimeoutError):
            _ = await wait_for(probe.wait(), timeout=0.05)

    run(scenario())


def test_cancelling_task_future() -> None:
    """Cancelling the Python task awaiting a pending binding call must drop
    the underlying Rust future -- not merely raise `CancelledError` in
    Python while leaving the Rust side running undetected."""

    async def wait_until_dropped(probe: _PendingProbe) -> None:
        while not probe.dropped:
            await sleep(0.001)

    async def scenario() -> bool:
        probe = _PendingProbe()
        waiting = ensure_future(probe.wait())
        # Yield a few times so the task actually runs and its Rust future
        # is polled at least once before it gets cancelled.
        for _ in range(3):
            await sleep(0)
        _ = waiting.cancel()
        with pytest.raises(CancelledError):
            await waiting
        # The drop itself runs on the tokio runtime's own worker thread,
        # woken by (not synchronous with) the Python-side cancellation:
        # poll briefly instead of assuming it lands within one loop tick.
        _ = await wait_for(wait_until_dropped(probe), timeout=1.0)
        return probe.dropped

    assert run(scenario()) is True


# === Private Functions === #

# === Entry Point === #
