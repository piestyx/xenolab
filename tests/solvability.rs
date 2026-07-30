use std::collections::BTreeSet;

use xenolab::engine::contamination::ContaminationLevel;
use xenolab::engine::ids::{NodeId, ObjectiveId};
use xenolab::engine::interventions::Intervention;
use xenolab::engine::run::{RunFailure, RunStatus};
use xenolab::engine::runlog::hash_events;
use xenolab::engine::sim::Simulator;
use xenolab::worldgen::generate_playable;
use xenolab::worldgen::spec::Archetype;

#[derive(Debug, Clone, Copy)]
struct CorpusCase {
    seed: u64,
    archetype: Archetype,
    objective: ObjectiveId,
}

const CORPUS: [CorpusCase; 15] = [
    CorpusCase {
        seed: 15,
        archetype: Archetype::UvSensitive,
        objective: ObjectiveId::StabilizePlant,
    },
    CorpusCase {
        seed: 10,
        archetype: Archetype::UvSensitive,
        objective: ObjectiveId::Detox,
    },
    CorpusCase {
        seed: 5,
        archetype: Archetype::UvSensitive,
        objective: ObjectiveId::PreventCollapse,
    },
    CorpusCase {
        seed: 21,
        archetype: Archetype::NutrientLimited,
        objective: ObjectiveId::StabilizePlant,
    },
    CorpusCase {
        seed: 1,
        archetype: Archetype::NutrientLimited,
        objective: ObjectiveId::Detox,
    },
    CorpusCase {
        seed: 11,
        archetype: Archetype::NutrientLimited,
        objective: ObjectiveId::PreventCollapse,
    },
    CorpusCase {
        seed: 12,
        archetype: Archetype::ToxinDriven,
        objective: ObjectiveId::StabilizePlant,
    },
    CorpusCase {
        seed: 7,
        archetype: Archetype::ToxinDriven,
        objective: ObjectiveId::Detox,
    },
    CorpusCase {
        seed: 2,
        archetype: Archetype::ToxinDriven,
        objective: ObjectiveId::PreventCollapse,
    },
    CorpusCase {
        seed: 18,
        archetype: Archetype::SymbiosisFragile,
        objective: ObjectiveId::StabilizePlant,
    },
    CorpusCase {
        seed: 13,
        archetype: Archetype::SymbiosisFragile,
        objective: ObjectiveId::Detox,
    },
    CorpusCase {
        seed: 8,
        archetype: Archetype::SymbiosisFragile,
        objective: ObjectiveId::PreventCollapse,
    },
    CorpusCase {
        seed: 39,
        archetype: Archetype::DetoxEcosystem,
        objective: ObjectiveId::StabilizePlant,
    },
    CorpusCase {
        seed: 4,
        archetype: Archetype::DetoxEcosystem,
        objective: ObjectiveId::Detox,
    },
    CorpusCase {
        seed: 14,
        archetype: Archetype::DetoxEcosystem,
        objective: ObjectiveId::PreventCollapse,
    },
];

fn completion_policy(case: CorpusCase) -> Simulator {
    let mut sim = Simulator::new(generate_playable(case.seed));
    let action = match case.objective {
        ObjectiveId::StabilizePlant => Intervention::SetUvHigh,
        ObjectiveId::Detox => Intervention::NeutralizeToxin(20.0),
        ObjectiveId::PreventCollapse => Intervention::AdvanceTime,
    };
    for _ in 0..30 {
        if sim.apply(action.clone()).is_err() || sim.run_state().status != RunStatus::Active {
            break;
        }
    }
    sim
}

fn reckless_policy(case: CorpusCase) -> Simulator {
    let mut sim = Simulator::new(generate_playable(case.seed));
    let actions = if case.objective == ObjectiveId::PreventCollapse {
        std::iter::once(Intervention::RemoveBacteria)
            .chain((0..20).map(|_| Intervention::SterilizeSample))
            .collect::<Vec<_>>()
    } else {
        (0..20)
            .map(|_| Intervention::AddToxin(20.0))
            .collect::<Vec<_>>()
    };
    for action in actions {
        if sim.apply(action).is_err() || sim.run_state().status != RunStatus::Active {
            break;
        }
    }
    sim
}

fn observed_trial(seed: u64, contamination: bool) -> (f32, f32) {
    let mut sim = Simulator::new_for_analysis(generate_playable(seed));
    if contamination {
        for _ in 0..10 {
            sim.apply(Intervention::AddToxin(0.0)).unwrap();
        }
    }
    let before = sim.state().get(NodeId::BacteriaPop);
    sim.apply(Intervention::AddToxin(20.0)).unwrap();
    let event = sim.apply(Intervention::ScanPopulation).unwrap();
    let measured = event
        .measurements
        .iter()
        .find(|measurement| measurement.node == NodeId::BacteriaPop)
        .map(|measurement| measurement.measured_value)
        .unwrap();
    (sim.state().get(NodeId::BacteriaPop) - before, measured)
}

#[test]
fn corpus_covers_archetypes_and_objectives_with_matching_metadata() {
    let mut archetypes = BTreeSet::new();
    let mut objectives = BTreeSet::new();
    for case in CORPUS {
        let recipe = generate_playable(case.seed);
        assert_eq!(
            recipe.archetype, case.archetype,
            "archetype drift for {}",
            case.seed
        );
        assert_eq!(
            recipe.objective, case.objective,
            "objective drift for {}",
            case.seed
        );
        assert!(recipe
            .initial_state
            .values
            .iter()
            .all(|value| value.is_finite() && (0.0..=100.0).contains(value)));
        archetypes.insert(recipe.archetype);
        objectives.insert(recipe.objective.label());
    }
    assert_eq!(archetypes.len(), 5);
    assert_eq!(objectives.len(), 3);
}

#[test]
fn completion_policies_win_within_the_action_ceiling() {
    let mut action_counts = Vec::new();
    for case in CORPUS {
        let sim = completion_policy(case);
        assert_eq!(
            sim.run_state().status,
            RunStatus::Won,
            "policy failed for {:?}",
            case
        );
        assert!((3..=30).contains(&sim.run_state().actions_used));
        assert!(sim.debrief().is_some());
        action_counts.push(sim.run_state().actions_used);
    }
    assert!(action_counts.iter().any(|count| *count >= 12));
}

#[test]
fn reckless_policies_preserve_failure_pressure() {
    for case in CORPUS {
        let sim = reckless_policy(case);
        assert!(
            matches!(
                sim.run_state().status,
                RunStatus::Failed(RunFailure::ContainmentLost)
                    | RunStatus::Failed(RunFailure::ActionBudgetExhausted)
            ),
            "reckless policy unexpectedly remained active for {:?}",
            case
        );
    }
}

#[test]
fn direct_intervention_is_meaningful_and_runs_are_deterministic() {
    for case in CORPUS {
        let recipe = generate_playable(case.seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe.clone());
        let before = *sim.state();
        sim.apply(Intervention::SetUvHigh).unwrap();
        assert_ne!(
            sim.state().get(NodeId::UvLevel),
            before.get(NodeId::UvLevel)
        );

        let a = completion_policy(case);
        let b = completion_policy(case);
        assert_eq!(a.debrief(), b.debrief());
        assert_eq!(hash_events(a.events()), hash_events(b.events()));
    }
}

#[test]
fn causal_signal_remains_visible_as_scan_fidelity_degrades() {
    let stable = observed_trial(7, false);
    let compromised = observed_trial(7, true);
    assert!(stable.0 < -0.5);
    assert!(compromised.0 < -0.5);
    assert!(compromised.1.is_finite());

    let mut sim = Simulator::new_for_analysis(generate_playable(7));
    for _ in 0..10 {
        sim.apply(Intervention::AddToxin(0.0)).unwrap();
    }
    let event = sim.apply(Intervention::ScanPopulation).unwrap();
    let measurement = &event.measurements[0];
    assert_eq!(
        measurement.contamination_multiplier,
        ContaminationLevel::Compromised.noise_multiplier()
    );
    assert!(measurement.total_multiplier >= 1.5);
    assert!(measurement.effective_sigma > 0.0);

    let mut critical = Simulator::new_for_analysis(generate_playable(7));
    for _ in 0..15 {
        critical.apply(Intervention::AddToxin(0.0)).unwrap();
    }
    let critical_event = critical.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(
        critical_event.measurements[0].contamination_multiplier,
        ContaminationLevel::Critical.noise_multiplier()
    );
}
