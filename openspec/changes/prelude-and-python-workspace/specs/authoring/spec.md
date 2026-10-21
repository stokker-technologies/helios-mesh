## ADDED Requirements

### Requirement: Feature-Aware Prelude

The facade SHALL provide `helios_mesh::prelude` with shared protocol exports,
host exports under `host`, firmware exports under `firmware`, and board exports
only under `firmware` on ARM. Existing public paths SHALL remain available.

#### Scenario: Consumer Selects Features

- **WHEN** a consumer compiles with all features or without default features
- **THEN** the prelude exports the types available for that feature selection
- **AND** exported types retain their identity through existing public paths

### Requirement: Root Python Workspace

The repository root SHALL own the uv workspace and lockfile, with `tools/python`
as a member and the Python distribution excluded. Tasks SHALL use the root
environment without redirecting `UV_PROJECT_ENVIRONMENT`. Python lanes SHALL
use their starting interpreter through the Hub-rendered runner.

#### Scenario: Tasks Run From A Sibling Worktree

- **WHEN** repository checks and Python lanes run in a sibling worktree
- **THEN** uv resolves the tooling from that worktree's root lockfile
- **AND** Python lanes consume the built distribution through the root environment
