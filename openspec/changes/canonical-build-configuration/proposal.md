# Canonical Build Configuration

## Why

Ordinary native Build currently executes broad feature and cross-target compatibility passes. The user now requires provider CI to cover only canonical shipping configurations.

## What Changes

Preserve shipping behavior and mandatory native platforms. Canonical Product/Tests share compatible Cargo outputs and matching Clippy settings. Move noncanonical feature combinations and test instrumentation, if present, to explicit optional local extended validation. Preserve all test sources and identify the exact coverage moved. Do not remove actual shipping firmware targets merely because they are cross-compiled.

## Impact

Repository mise tasks, build/test producers, affected configuration/tests and maintained specs/README. Shared tooling and skills remain Hub-owned.
