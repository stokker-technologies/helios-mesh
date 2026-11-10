# GPIO Pulse Level Design

Status: proposed on 2026-11-10; not locked.

## Shape

```text
gpio-pulse <id> high|low <ms>
```

```rust
sidecar.pulse_gpio(AuxGpioId(1), AuxGpioLevel::High, Duration::from_millis(1000)).await?;
```

```python
await sidecar.pulse_gpio(AuxGpioId(1), AuxGpioLevel.HIGH, timedelta(seconds=1))
```

`PulseAuxGpio` gains a level field next to `gpio` and `duration_ms`. The node
drives the opposite level when it accepts the request, the pulse level for the
duration on its local timer, and the opposite level again.

## Open Decisions

1. **Rest state after the pulse.** The pin stays driven at the opposite level
   (recommended: the line never floats and a following pulse starts from a
   known level), or it returns to the mode it had before the pulse.
2. **Level argument.** Required with no default (recommended: the caller
   states the polarity, so a wiring assumption cannot hide), or optional with
   a default level.
