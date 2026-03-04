# xenolab

Prototype terminal research roguelike focused on deterministic world generation,
simulation, and replayable run logs.

## v0.1 Scope

- Deterministic world generation from seed
- Deterministic simulation and interventions
- In-memory runlog with deterministic hash
- Minimal ratatui terminal UI
- Acceptance tests for worldgen and replay determinism

## Build, Run, Test

- `cargo run`
- `cargo test`

## Determinism

World generation and simulation are seed-based and deterministic by design.

## Structure Overview

- `src/engine`: core simulation data model and runtime
- `src/worldgen`: recipe generation and acceptance harness
- `src/ui`: minimal ratatui application shell
- `tests/`: acceptance and determinism integration tests
