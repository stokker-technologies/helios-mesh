# Prelude And Python Workspace

## Why

Host applications and firmware import a dozen `helios_mesh` items by name, and the Python tooling environment was pinned through `UV_PROJECT_ENVIRONMENT`, which redirects every uv invocation and breaks worktrees.

## What Changes

- `helios_mesh::prelude` re-exports the shared protocol types, host discovery and typed handles, and the firmware authoring types behind their features; existing paths stay.
- The repository root becomes the uv workspace (`tools/python` as member, the distribution excluded) and owns `uv.lock`; the tooling stops redirecting `UV_PROJECT_ENVIRONMENT` and runs tests with the interpreter it was started with.
- Python line length 140; the barrel check excludes the prelude by declaration.

## Impact

Consumers may shorten imports; no public item moves. README and crate README document the prelude and the workspace layout.
