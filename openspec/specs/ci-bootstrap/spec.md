# CI Bootstrap Specification

## Purpose
Declare Linux native prerequisites for pinned product tools before CI installs them, preserving cold-run behavior across hosted and local runners.

## Requirements

### Requirement: Native Product Tool Prerequisites
Every Linux CI job installing the pinned elf2uf2 tool SHALL provision its native build prerequisites before mise installation through the declared job bootstrap. Hosted and local runners SHALL execute the same setup and retain all existing tool features and required gates.

#### Scenario: Cold Linux Runner
- **WHEN** a selected Linux job starts without libudev development headers or pkg-config
- **THEN** bootstrap installs the prerequisites before mise builds elf2uf2, and the required job continues without reliance on a preconfigured local image.
