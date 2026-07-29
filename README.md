# xenolab

`xenolab` is a prototype terminal research roguelike in Rust. The current implementation
baseline is tagged `v0.1.4`: it generates a deterministic causal micro-ecosystem from a seed,
lets you intervene in that world, and records run events with deterministic hashing.

The v0.2.0 implementation adds a bounded run lifecycle with deterministic win/failure resolution,
terminal lockout, debrief data, and same-seed/new-seed restart flow. See the [v1.0 completion
contract](docs/V1_COMPLETION_CONTRACT.md) and [v1.0 baseline audit](docs/V1_BASELINE_AUDIT.md).

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

## Archetypes and Legibility

- Each seed deterministically maps to an interaction archetype:
  `UvSensitive`, `NutrientLimited`, `ToxinDriven`, `SymbiosisFragile`,
  or `DetoxEcosystem`.
- Archetypes bias which constraints dominate plant outcomes (UV chain,
  nutrient depletion, toxin pressure, fragile symbiosis, or detox loops).
- Generated graphs are intentionally sparse (`6..=8` edges) with per-node
  incoming-degree caps and tiered edge magnitudes (primary, secondary, spice)
  to keep causal structure readable.
- Some archetypes may include a deterministic UV-toxin threshold hook
  (`Burn` or `Create`) for rare nonlinear behavior.

## Run Loop

- The app starts with a generated playable world for the selected seed (default `42`).
- Each run has 30 budget-consuming actions, shown as actions remaining in Lab.
- In Lab, selecting an intervention and pressing `Enter` applies it through the engine.
- Every non-scan intervention advances simulation by one tick automatically.
- Scan interventions capture measurements without advancing time but consume one action.
- Objective progress is evaluated against true state after every accepted action. `StabilizePlant`
  requires plant >= 60 for 3 consecutive evaluations; `Detox` requires toxin <= 15 for 3;
  `PreventCollapse` requires plant and bacteria >= 25 for 3.
- Completing the objective produces a win. Using all 30 actions without success produces an
  `ActionBudgetExhausted` failure. Resolved runs reject further simulation actions.
- Runlog entries capture intervention, measurement data, tick, contamination, and state snapshot.

## Debrief and Restart

- A resolved run shows its outcome, final state, objective progress, action usage, and deterministic
  run-event hash.
- `r`: restart with the same seed.
- `n`: enter a new decimal `u64` seed, then press `Enter`; `Esc` cancels seed entry.
- `q`: quit from the active run or debrief.

## Tabs

- `Lab`: live status, world metrics with deltas, and intervention actions
- `Journal`: scenario text, objective narrative, rules, and controls
- `Log`: chronological run events

## Objective Notes

- The seed selects one of three objective descriptions: `StabilizePlant`, `Detox`, or
  `PreventCollapse`.
- Objective progress is engine-owned and shown in Lab as consecutive qualifying evaluations.
- A resolved objective or exhausted action budget moves the UI to the debrief.

## Controls

- `q`: quit
- `1`: lab view
- `2`: journal view
- `3`: log view
- `Up`/`Down`: navigate Lab actions, scroll Journal, or scroll Log
- `j`/`k`: scroll Journal
- `PageUp`/`PageDown`: fast-scroll Journal
- `Enter`: apply selected intervention in Lab
- `r`: restart same seed after resolution
- `n`: enter a new seed after resolution

## Structure Overview

- `src/engine`: core simulation data model and runtime
- `src/worldgen`: recipe generation and acceptance harness
- `src/ui`: minimal ratatui application shell
- `tests/`: acceptance and determinism integration tests
