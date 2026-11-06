# Canonical Build Configuration

## Profiles And Artifacts

Product uses Cargo's built-in dev or release profile. Tests and their matching Clippy use unoptimized test in both graph contexts; Bench uses bench. No workspace or package compiler-profile overrides are retained.

The requested graph profile names exports and reports. Product and Tests are independent producers. Package consumes Product bytes without compiling; native Rust lanes consume the matching Tests archive, and Python acceptance consumes the built native wheel.

## Native Validation

Linux and Windows retain equal native coverage. Linux also cross-compiles every RP2040 image. Canonical shipping features remain separate from explicit compatibility and instrumented transport proof.

The compatibility task checks noncanonical feature combinations. The extended task compiles instrumented transport stories without changing shipping artifacts. Ordinary integration includes every canonical-capable story.

## Current Delivery Policy

The harmonize-conventions change supersedes the earlier generated CI wiring and local simulation requirement. Native project-owned workflows implement quick ordinary dev checks and builds, PR test and package jobs, and exact-commit stable release gates. Existing provider protections remain in force.

Machine-wide compilation and native tests share an operating-system build lock with wait diagnostics and crash recovery. Independent worktrees retain independent Cargo targets and use the capped shared local compiler cache. Independent simulation jobs follow dependency order concurrently.
