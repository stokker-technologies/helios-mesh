# Repository Instructions

Helios Node Mesh is a Stokker Technologies Rust and RP2040 firmware project.

- Apply `stkr-conventions`, `stkr-mise`, and the applicable Rust, CLI, Python,
  PyO3, testing, CI, and publishing skills. Use repository mise tasks.
- Preserve the host contracts in `crates/system/hnm/README.md` and the hardware,
  wiring, and firmware contracts in `crates/firmware/hnm-board-feather-rp2040/README.md`.
- Preserve wire version 14 and its encoding. Protocol changes require an explicit
  decision, a version bump, and matching firmware on all participating boards.
- Keep workspace packages and firmware synchronized. The migration remains at
  0.1.0; release preparation is patch-only, beginning with stable v0.1.0.
- Cargo metadata owns Apache-2.0 licensing. Native manifests own dependencies;
  mise owns profile arguments, build, tests, and package recipes.
- Build once for the selected profile; lanes consume the archive and wheel.
  Host archives exclude ARM binaries. Linux cross-compiles all RP2040 images.
- GitHub is authoritative. GitLab is a one-way mirror with CI disabled. Native
  Linux and Windows results are both required; neither certifies physical RF,
  electrical behavior, or macOS.
- Keep private specifications, machine state, and temporary diagnostics in their
  allowed ignored locations. `.agents/` is canonical source, never private state.
- Use Conventional Commits and run `mise run check` plus local deterministic
  checks before each authorized push. SOURCE COMPLETE precedes full validation.
