# Contributing

## Development Setup

Start with the [Quick Start](README.md#quick-start), the pinned mise toolchain,
and the native prerequisites it lists.

After [Quick Start](README.md#quick-start), choose a repository task:

| Command | Purpose |
| --- | --- |
| `mise run check` | Check Hub conventions, formatting, dependency hygiene, Python lints and types, and workflow syntax |
| `mise run build --profile dev` | Build shipping binaries, wheels, and Linux firmware with matching Clippy |
| `mise run build:rust:tests --profile dev` | Run canonical matched Clippy and build the independent Rust test archive |
| `mise run check:python:generated --profile dev` | Verify the built wheel against Python stubs |
| `mise run test` | Run Rust unit, integration, acceptance, and Python acceptance lanes |
| `mise run package --profile dev` | Assemble existing CLI, wheel, and firmware artifacts |

Build outputs live under `target/build/<profile>/<target>/`, package outputs under
`target/package/<profile>/<target>/<language>/`, and reports under
`target/tests/<profile>/<target>/`; the examples use `linux-x86_64`, and Windows
uses `windows-x86_64`. Tests consume the matching test archive from
`mise run build:rust:tests` and the installed native wheel. The root
`.venv` is shared by local tasks, which run sequentially; native CI jobs use
isolated workspaces. The root uv workspace includes `tools/python` and owns
`uv.lock`; the Python distribution keeps its independent manifest and lockfile.
Required public CI uses repository-native tooling.


## Development Workflow

Use a Conventional Commit task branch from `dev` and keep the complete change in one integration branch.
Close the agreed source inventory before running the final candidate validation.
Prose-only changes select lightweight repository checks through the declared impact policy;
executable examples and mixed source changes retain their affected proof.

Provider CI runs the native graph on every configured platform.
Background validation keeps Windows child consoles hidden while preserving
command output and exit status. Interactive product commands keep their normal
console behavior.

Every GitHub CI job uses a platform-matching GitHub-hosted runner. Separate
Product and Tests Build jobs run in parallel, with independent canonical feature
sets. Each compilation follows Clippy with matching packages, features, target
and profile. Tests Build needs no Product outputs; test lanes receive the
matching archive and Product artifacts they require. Package consumes Product
independently of Tests. Linux Product also
cross-compiles all three shipped RP2040 firmware images.

Product uses Cargo's default `dev` or `release` profile, Bench uses default
`bench`, and Tests and their matching Clippy use default unoptimized `test`
in both graph profiles. The requested graph profile still names the artifacts
and reports. Workspace and package compiler-profile overrides are absent.

Ordinary CI and local validation use shipping features. The optional local
`mise run build:rust:compatibility` checks six additional feature combinations,
including all-features and no-default-features SDK variants. On either native
platform, `mise run test:rust:extended` builds and runs the 28 instrumented
transport tests with the `testing` feature, writing Nextest JUnit under
`target/tests/<profile>/<target>/extended-rust/store/stkr/`. The extended task accepts `--profile release` to select the report namespace;
neither runs automatically in CI or ordinary validation.
The compatibility task also cross-checks ARM binaries and needs the declared
`thumbv6m-none-eabi` Rust target. These opt-in tasks add compilation cost and
reuse the workspace Cargo directory without replacing the canonical archive.
Ordinary CI jobs default to 120 minutes while individual test deadlines remain
unchanged. Pull requests can write compatible caches within their native merge-ref
scope. New PR and ordinary `dev` commits cancel superseded validation while
release tagging and publication retain their required handoff.
CI prepares native compiler and archive tools before restoring caches; runner
slots use isolated mise stores. Lane artifacts retain native diagnostics beside
JUnit reports, including on failure, for seven days.

Linux and Windows native checks are both required. Hardware tests and
firmware flashing require an explicit decision and suitable physical setup.

## Tests And Checks

After source closure, run the impact-selected native review gate:

```sh
mise run ci:simulate
```

It runs the required Windows and Linux jobs from the same declared provider graph.
A platform-limited simulation is partial evidence and does not satisfy the review gate.

- [Host Contracts](crates/system/helios-mesh/README.md) — discovery, typed Rust/Python APIs, transport, and operation ownership.
- [Board And Firmware Guide](crates/firmware/helios-mesh-board-feather-rp2040/README.md) — wiring, safety, node authoring, and field diagnostics.
- [Python Examples](crates/system/helios-mesh-python/examples/) — Rotator sweep and tuning workflows.
- [Public Rust Facade](crates/system/helios-mesh/src/lib.rs) — the host and firmware API exports.
- [Wire Contract](crates/common/helios-mesh-protocol/src/lib.rs) — the shared versioned protocol.

## Pull Requests

Task changes target `dev`. The manual `Release / Prepare` job opens
`release/v<version>` into `dev`. After the required checks and release rehearsal,
the review squash-merges into `dev`; `Release / Tag` creates the stable tag,
publishes crates, and fast-forwards `main` to that commit. The tag workflow
publishes and verifies the native packages. Release notes come from Conventional
Commits at tag time. Downloads include the firmware ZIP, host binaries, wheels,
and `SHA256SUMS`; the Linux label on the firmware download names its build host,
while its contents target RP2040.

Use one pull request into `dev` and enable squash auto-merge after local validation.
Required Linux and Windows native checks must pass before integration.

## Reporting Issues

Bug reports, focused fixes, hardware observations, and examples are welcome.
[Open an issue](https://github.com/stokker-technologies/helios-mesh/issues)
to discuss a change. Include the revision, host platform, node role, attached
hardware, and relevant diagnostics when reporting a device problem.
