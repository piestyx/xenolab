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
- In Lab, selecting an intervention and pressing `Enter` applies it through the engine.
- Every non-scan intervention advances simulation by one tick automatically.
- Scan interventions capture measurements without advancing time.
- Runlog entries capture intervention, measurement data, tick, contamination, and state snapshot.

## Tabs

- `Lab`: live status, world metrics with deltas, and intervention actions
- `Journal`: scenario text, objective narrative, rules, and controls
- `Log`: chronological run events

## Objective Notes

- `StabilizePlant`: keep plant population high for a consecutive tick window.
- `Detox`: drive toxin low and hold it across consecutive ticks.
- `PreventCollapse`: keep plant and bacteria above safety thresholds together.

## Controls

- `q`: quit
- `1`: lab view
- `2`: journal view
- `3`: log view
- `Up`/`Down`: navigate Lab actions, scroll Journal, or scroll Log
- `j`/`k`: scroll Journal
- `PageUp`/`PageDown`: fast-scroll Journal
- `Enter`: apply selected intervention in Lab

## Structure Overview

- `src/engine`: core simulation data model and runtime
- `src/worldgen`: recipe generation and acceptance harness
- `src/ui`: minimal ratatui application shell
- `tests/`: acceptance and determinism integration tests
