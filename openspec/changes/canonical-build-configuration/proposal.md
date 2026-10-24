# Canonical Build Configuration

## Why

Independent parallel Product and Tests builds have landed, but custom compiler profiles remain. The user now requires Cargo's built-in defaults and unoptimized test compilation in every graph context.

## What Changes

Remove workspace, package and build-dependency profile overrides. Product selects built-in dev or release; Bench selects built-in bench; Tests and their preceding Clippy select built-in test even in a release graph. Keep artifact/report namespaces tied to the requested graph profile. Preserve all current shipping features, firmware targets, canonical tests and explicit local extended coverage. No harness consolidation or new test architecture is included.

## Impact

Cargo manifests, authored test/compatibility tooling, relevant mise declarations, shared generated compiler helpers and maintained README/specs. Shared templates, checkers and guidance remain Hub-owned under coordinated task 6.39.
