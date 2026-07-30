# xenolab

`xenolab` is a deterministic terminal research game. You investigate a seeded
micro-ecosystem with interventions and noisy instruments, record causal
hypotheses, publish evidence, and spend earned credits on temporary repairs.

The current release-candidate target is `v0.9.0`; the v1.0 scope authority is
the [completion contract](docs/V1_COMPLETION_CONTRACT.md), supported by the
[baseline audit](docs/V1_BASELINE_AUDIT.md). See the
[demonstration seed guide](docs/DEMONSTRATION_SEEDS.md) for spoiler-free
examples.

## Build and run

```bash
cargo run -- 42
cargo test --offline
```

The optional argument is a decimal `u64` seed. If omitted, the application
uses seed `42`. A new seed can be entered after a run resolves; whitespace is
accepted, but negative, empty, mixed, and overflowing values are rejected.

The supported terminal size is at least **80 columns × 24 rows**. Smaller
terminals show a safe resize message and still accept `q`.

## Game loop

Each run begins active with a deterministic world, objective, and 30-action
budget. Interventions usually advance one simulation tick. Population and
chemical scans consume one action but do not advance time.

Objective progress is engine-owned and requires three consecutive qualifying
evaluations:

- `StabilizePlant`: plant population at least 60.
- `Detox`: toxin concentration at most 15.
- `PreventCollapse`: plant and bacteria populations both at least 25.

A failed evaluation resets the consecutive hold. Completing the objective wins.
Using the final action without success fails through action exhaustion.

## Contamination and instruments

Contamination is persistent and action-driven:

| Action | Base contamination |
| --- | ---: |
| scans, Advance Time, UV changes | 0 |
| Add nutrient | +1 |
| Add toxin | +2 |
| Neutralise toxin, remove fungus/bacteria | +1 |
| Sterilise sample | +3 |

Levels are Stable `0–19`, Compromised `20–29`, Critical `30–39`, and Lost at
`40`. Compromised scans use `1.5×` noise and Critical scans use `2.25×` noise.
Contamination does not alter true state or objective evaluation. An objective
win takes precedence if it occurs on the same action as containment loss.

## Notebook, publication, and repairs

The Notebook stores up to eight templated hypotheses: `X increases Y` or
`X decreases Y`, using six observable variables. Notebook edits consume no
actions, ticks, contamination, or RNG and are active-run only.

Publishing a hypothesis costs one action and is permanent. It uses only direct
intervention evidence followed by the appropriate scan. Results are
`Unsupported`, `Weak`, `Moderate`, or `Strong`, awarding `0`, `1`, `2`, or `3`
run-local credits. There are at most four publications and twelve credits.

The Repairs view spends current-run credits without consuming actions, ticks,
contamination, or RNG:

- Calibration levels 0–2 cost 2 then 4 credits and reduce future scan noise to
  `1.00×`, `0.80×`, then `0.60×`.
- Containment levels 0–2 cost 2 then 4 credits and reduce future contamination
  costs by `0`, `1`, then `2`.

Repairs affect only future operations. Credits, Notebook state, publications,
and repairs reset on restart.

## Tabs and controls

- `1` Lab: Up/Down select an action, `Enter` applies it, `x` repeats the last
  accepted intervention or scan.
- `2` Journal: Up/Down, `j`/`k`, and PageUp/PageDown scroll guidance.
- `3` Log: Up/Down scroll; `a` all, `i` interventions, `m` measurements,
  `p` publications, `r` repairs.
- `4` Notebook: `a` add, `e` edit, `d` delete, `p` publish; Enter confirms and
  Esc cancels.
- `5` Repairs: Up/Down select a track, Enter requests purchase confirmation,
  Esc cancels.
- `?` opens the complete in-game control reference.
- `q` quits from active play, views, dialogs, and the debrief.
- After resolution, `r` restarts the same seed and `n` opens new-seed entry.

Resolved runs lock simulation, Notebook, publication, and repair mutations.
The debrief remains readable and includes outcome, final state, hashes,
research records, and repair history.

## Determinism and replay

World generation, simulation RNG, run events, and complete operation sequences
are deterministic for the same seed and inputs.

`hash_events` is the gameplay event hash. It covers the append-only simulation
events and intentionally excludes Notebook, publication, and repair records.
The separate `verification_hash` uses the schema tag
`xenolab-verification-v1` and hashes, in stable serialized order, the seed,
recipe hash, gameplay event hash, Notebook, publications, wallet values,
repair levels and purchases, run state, and debrief. It verifies the complete
outcome-relevant in-memory run history.

WP08 provides typed in-memory replay operations and verification tests. It does
not provide replay files, import/export, persistence, or a verifier command.

## Current limitations

- Credits have no use beyond the two run-local repair tracks.
- There is no save/load, replay file format, campaign progression, or account
  progression.
- Objective and evidence balance remains subject to later balance review;
  deterministic corpus results are documented by WP07.

## Project structure

- `src/engine`: simulation, lifecycle, research records, replay, and hashes.
- `src/worldgen`: deterministic recipes, archetypes, and acceptance checks.
- `src/ui`: ratatui application, views, controls, and terminal handling.
- `tests`: lifecycle, determinism, worldgen, research, UI, solvability, and
  replay acceptance coverage.
