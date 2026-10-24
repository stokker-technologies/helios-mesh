# Canonical Build Design

## Parallel Producer Decision

Product and Tests now use separate parallel provider jobs, superseding single-job consolidation. Independent feature sets are permitted, but CI remains canonical-only and extended configurations remain local opt-in. Tests Build must take no longer than Product Build on the same candidate/platform/profile; measured breaches require investigation and repair of redundant compilation, feature sprawl or testing structure, not silent coverage removal or raised timeouts. Preserve actual artifact dependencies and report relocated work in the full critical path. Repository owner must reconcile obsolete single-job clauses before closure. The current Dokime pipeline remains uninterrupted.

## Locked Contract

All provider contexts execute only canonical shipping configurations. Mise owns features per platform/profile. Separate Product and Tests provider jobs run in parallel with independent canonical feature sets. Local sequential builds reuse the workspace Cargo directory where compatible. Tests does not depend on Product output. Clippy precedes each distinct compilation with matching settings. Installer tasks remain Product-only. Use only built-in Cargo profiles: Product dev/release, Bench bench, and Tests plus their preceding Clippy unoptimized test regardless of graph profile. The former release-tests exception and all authored profile tables are removed.

build:rust:compatibility is an explicit local compiler-matrix helper. test:rust:extended is the optional local behavioral suite and owns its required builds; neither is inferred into CI or ordinary local validation. Avoid multi-archive machinery. Preserve firmware shipping configurations; distinguish unsupported feature combinations from actual shipped targets before moving checks. No shipped SDK feature expansion to make tests fit.

## Ownership

Existing Helios owner implements in the current migrate-hub-ci worktree. The parallel-build migration merged; the explicit Default Cargo Profiles follow-up uses fix/default-cargo-profiles from merged dev in the same worktree and rebinds this applicable change. Coordinate with parent and Hub owner on managed output changes. Targeted proof until source closure; then full available local proof and protected PR into dev with both required native provider results. No release publication.

## Source And Coverage Inventory

- `build/__main__.py` owns the Product task; `build/rust/tests.py` owns canonical archive selection; `build/rust/compatibility.py` and `test/rust/extended.py` own the opt-in checks. `check/python/generated.py` owns binding verification. Mise entry points mirror these module paths without adding forwarding modules; `package/product.py` remains a genuine shared language helper.
- Six compatibility selectors move local: workspace excluding nodes with all/no-default features; facade all-targets with all/no-default features; nodes library with all features; ARM node binaries with all features.
- Two old checks duplicate canonical proof and are removed: nodes library without defaults (its defaults are empty), and ARM node binaries without defaults plus firmware (the Linux shipping Product selection).
- All 28 tests in `crates/system/helios-mesh/tests/transport/` remain unchanged and move from the ordinary archive to the direct local extended runner with `testing`. No second archive is introduced.
- Canonical coverage retains all default unit tests, noninstrumented integration tests, CLI acceptance, Python acceptance including the three existing private bridge-seam tests, and Linux cross-compilation of the three RP2040 images. Default unit mocks and the shipped Python seam are unchanged by explicit parent decision.
- `README.md` documents opt-in cost and hosted runners. Existing native CI and authoring specifications retain their shipped behavior contracts; this change adds canonical-only provider selection.
- No benchmark tasks or feature combinations currently exist, so no extended benchmark task is needed. Managed helper inference, Cargo API migration and generated copies remain Hub-owned and require the validated handoff before source closure.
The previous full native proof counted 315 tests: 215 Rust unit, 56 Rust integration, 3 Rust acceptance and 41 Python acceptance. Moving transport's 28 leaves an expected 287 canonical cases and 28 explicit extended cases; final native reports establish actual counts.

## Parallel Build And Performance Acceptance

Tests stamps its own archive and compiler metadata rather than requiring Product CLI/wheel files. CLI acceptance uses the binary that Nextest includes through Cargo binary metadata; Python and generated-binding checks consume the independently built wheel downstream. No new fixture framework is needed.

On the same candidate, native platform and profile, Tests Build duration MUST be no greater than Product Build duration. Every warm full pipeline MUST finish in less than 45 minutes, with a 30-minute target. A regression requires reassessment; do not silently drop coverage, alter architecture or change shipping features. Prior successful review duration was 13m59; Windows Product 2m45 versus Tests 4m48 requires new measured proof after the compatibility cutover. Linux Product 4m16 versus Tests 2m07 already met the comparison. Provider proof must measure the new separate jobs.

## Declared Artifact Ownership

stkr.toml declares Product bin/wheel/target metadata plus Linux firmware, independent Tests archive/stamp, and native package outputs. Rust unit/integration/acceptance consume only Tests (Nextest carries its CLI); Python acceptance/generated checks consume Product. Packages consume Product only. Coverage receives its required base inputs. Dry/Rehearsal/Publish aggregate package outputs across platforms. Verify consumes only remote published assets, checking their SHA-256 and signatures before native installation, so it has no local package input. Inputs express both waiting and transfer; no duplicate ordering-only depends edges are added. Consumers use cargo.archive, cargo.record_target and cargo.artifacts; legacy managed APIs are retired by canonical generation.

## Default Compiler Profile Inventory

Remove the five root Cargo profile tables: release, release-tests, dev, dev package wildcard and bench; no other manifest declares profiles. Preserve the empty canonical Profiles section heading. Product selectors and graph namespaces stay dev/release, and the shared Hub archive/coverage/bench helpers receive canonical profile mapping from the coordinated snapshot. Authored build/rust/compatibility and test/rust/extended use compiler test. Compatibility has no artifacts and drops its redundant graph-profile option; extended keeps the requested report namespace. No shipped features, test cases or dependencies change. The README and this active change supersede the former custom release-tests exception. All prior implementation items reopen until the mapping and generated helper/checker adoption close; provider timing needs evidence from the new candidate.

## Landing Page And Prose Proof

The root README keeps CLI, Python, Rust and firmware examples under their shipped interfaces; CONTRIBUTING owns setup, validation and contribution detail. Existing executable example blocks remain unchanged. Registry badges state unpublished and route to source setup until real registry owners exist. An approved logo is required; no product identity asset is invented.

Explicit stkr.toml prose rules select check:repository for root README/CONTRIBUTING and audited docs Markdown, excluding examples, tutorials, templates and site code. LICENSE selects package:rust as well because package/product.py embeds it in the firmware ZIP. The canonical shared resolver unions known per-path selections, closes producer dependencies and uses full proof for unknown/protected changes. Provider protection reconciliation to the stable aggregate remains a delivery action, separate from credential-disabled rendering.

## Runtime Guard And Artifact Follow-Up

PR 28 merged into dev at 07d4d71e5607198ff74ded125cf710fb7845cffe. Its later warm-cache replay failed before Linux unit tests started: successful artifact download was followed by zstd SIGPIPE during streaming tar extraction. The shared extractor must drain decompressed trailing bytes after tar end-of-archive, then require a successful decompressor exit. Corrupt streams still fail; no SIGPIPE suppression or retry is introduced.

Continue in fix/test-runtime-guards from that merged dev in the same owned worktree. Adopt the coordinated Hub repair and minimum 30-second correctness-test hang guards, preserving higher declared budgets. Historical test durations do not tighten deadlines. The under-10-second application responsiveness target is investigated independently by phase; setup and artifact extraction are not application execution, and larger guards do not prove performance. Existing canonical and extended coverage remains unchanged. Final local validation and protected delivery remain open for this follow-up.

## Serial Local Simulation

Each local simulation runs one declared job at a time without thread, CPU or memory limits. Independent repository gates may run concurrently. Remove the shared resource allocator and its options rather than retaining a parallel compatibility path. Native Linux and Windows execution, declared artifact prerequisites, failure propagation and timing/cache evidence remain required. Provider jobs retain their declared graph; this decision changes local execution only.
