# xenolab

`xenolab` is a prototype terminal research roguelike in Rust that generates a deterministic
causal micro-ecosystem from a seed, lets you intervene in that world, and records replayable
run events with deterministic hashing for verification.

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

## Run Loop

- The app starts with a generated playable world for the selected seed (default `42`).
- Selecting an intervention and pressing `Enter` applies it through the engine.
- Every non-scan intervention advances simulation by one tick automatically.
- Scan interventions capture measurements without advancing time.
- Runlog entries capture intervention, measurement data, tick, contamination, and state snapshot.

## Controls

- `q`: quit
- `1`: status view
- `2`: console view
- `3`: log view
- `Up`/`Down`: navigate interventions (console) or scroll log (log view)
- `Enter`: apply selected intervention

## Structure Overview

- `src/engine`: core simulation data model and runtime
- `src/worldgen`: recipe generation and acceptance harness
- `src/ui`: minimal ratatui application shell
- `tests/`: acceptance and determinism integration tests
