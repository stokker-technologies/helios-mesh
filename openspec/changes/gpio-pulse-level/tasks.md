# GPIO Pulse Level Tasks

Status: proposed; not locked. No task starts before 1.1 closes.

## 1. Contract

- [ ] 1.1 Decide the two open decisions in `design.md` with the owner and record each outcome there.

## 2. Implementation

- [ ] 2.1 Add the level to `PulseAuxGpio` and its codec, and delete the fixed board pulse polarity.
- [ ] 2.2 Drive the pulse sequence in firmware per the decided rest state.
- [ ] 2.3 Add the level to the Rust client, the Python binding and stubs, and the CLI command; update the README.

## 3. Validation

- [ ] 3.1 Focused protocol, client, binding and CLI tests; firmware build and package on Linux.
- [ ] 3.2 Reflash a Sidecar and confirm a high and a low pulse on the bench.
