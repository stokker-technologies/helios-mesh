# Repository Instructions

Helios Mesh is a Stokker Technologies Rust and RP2040 firmware project.

- Use repository mise tasks and preserve project-owned automation.
- Preserve the host contracts in `crates/system/helios-mesh/README.md` and the hardware,
  wiring, and firmware contracts in `crates/firmware/helios-mesh-board-feather-rp2040/README.md`.
- Preserve wire version 14 and its encoding. Protocol changes require an explicit
  decision, a version bump, and matching firmware on all participating boards.
- Keep workspace packages and firmware synchronized. Nonbreaking releases increment the patch version; breaking changes before 1.0 increment the minor.
- Cargo metadata owns Apache-2.0 licensing. Native manifests own dependencies;
  mise owns profile arguments, build, tests, and package recipes.
- Build once for the selected profile; lanes consume the archive and wheel.
  Host archives exclude ARM binaries. Linux cross-compiles all RP2040 images.
- GitHub is authoritative. GitLab is a one-way mirror with CI disabled. Native
  Linux and Windows results are both required; neither certifies physical RF,
  electrical behavior, or macOS.
- Keep private specifications, machine state, and temporary diagnostics in their
  allowed ignored locations. `AGENTS.md` is hand-owned; `CLAUDE.md` references it.
- Use Conventional Commits. After SOURCE COMPLETE, run check tasks and targeted deterministic proof. Dev reviews squash immediately; stable tagging requires green required hosted CI on the exact release commit.
- Dev pushes run quick checks and Product Build; reviews run tests. Linux and Windows are equally required.
- Runtime policy lives in `tools/project.toml`; local authoring metadata lives in `.stkr/project.toml`.
