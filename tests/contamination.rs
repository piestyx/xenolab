use pretty_assertions::assert_eq;
use xenolab::engine::contamination::{
    ContaminationLevel, COMPROMISED_THRESHOLD, CONTAINMENT_LOST_THRESHOLD, CRITICAL_THRESHOLD,
};
use xenolab::engine::ids::NODE_COUNT;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::run::{RunFailure, RunStatus};
use xenolab::engine::sim::{SimError, Simulator};
use xenolab::engine::world::WorldState;
use xenolab::worldgen;

fn quiet_sim() -> Simulator {
    let mut recipe = worldgen::generate_playable(42);
    recipe.initial_state = WorldState {
        values: [50.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    recipe.biases = [0.0; NODE_COUNT];
    recipe.noise_sigma = [0.0; NODE_COUNT];
    Simulator::new(recipe)
}

#[test]
fn contamination_thresholds_are_central_and_exact() {
    assert_eq!(
        ContaminationLevel::from_value(0.0),
        ContaminationLevel::Stable
    );
    assert_eq!(
        ContaminationLevel::from_value(19.0),
        ContaminationLevel::Stable
    );
    assert_eq!(
        ContaminationLevel::from_value(COMPROMISED_THRESHOLD),
        ContaminationLevel::Compromised
    );
    assert_eq!(
        ContaminationLevel::from_value(29.0),
        ContaminationLevel::Compromised
    );
    assert_eq!(
        ContaminationLevel::from_value(CRITICAL_THRESHOLD),
        ContaminationLevel::Critical
    );
    assert_eq!(
        ContaminationLevel::from_value(39.0),
        ContaminationLevel::Critical
    );
    assert_eq!(
        ContaminationLevel::from_value(CONTAINMENT_LOST_THRESHOLD),
        ContaminationLevel::Lost
    );
    assert_eq!(
        ContaminationLevel::from_value(100.0),
        ContaminationLevel::Lost
    );
    assert_eq!(ContaminationLevel::Stable.noise_multiplier(), 1.0);
    assert_eq!(ContaminationLevel::Compromised.noise_multiplier(), 1.5);
    assert_eq!(ContaminationLevel::Critical.noise_multiplier(), 2.25);
}

#[test]
fn every_action_applies_the_central_contamination_cost_once() {
    let actions = [
        (Intervention::ScanPopulation, 0),
        (Intervention::ScanChemicals, 0),
        (Intervention::AdvanceTime, 0),
        (Intervention::SetUvLow, 0),
        (Intervention::SetUvHigh, 0),
        (Intervention::AddNutrient(0.0), 1),
        (Intervention::AddToxin(0.0), 2),
        (Intervention::NeutralizeToxin(0.0), 1),
        (Intervention::RemoveFungus, 1),
        (Intervention::RemoveBacteria, 1),
        (Intervention::SterilizeSample, 3),
    ];

    for (action, expected_cost) in actions {
        let mut sim = quiet_sim();
        sim.apply(action.clone()).unwrap();
        assert_eq!(sim.contamination(), expected_cost as f32, "{action:?}");
        assert_eq!(sim.peak_contamination(), expected_cost as f32, "{action:?}");
    }
}

#[test]
fn rejected_and_terminal_actions_do_not_change_contamination() {
    let mut sim = quiet_sim();
    assert!(matches!(
        sim.apply(Intervention::AddToxin(-1.0)),
        Err(SimError::InvalidDelta("AddToxin"))
    ));
    assert_eq!(sim.contamination(), 0.0);

    for _ in 0..14 {
        sim.apply(Intervention::SterilizeSample).unwrap();
    }
    assert_eq!(
        sim.run_state().status,
        RunStatus::Failed(RunFailure::ContainmentLost)
    );
    let contamination = sim.contamination();
    let events = sim.events().len();
    assert!(matches!(
        sim.apply(Intervention::AddToxin(0.0)),
        Err(SimError::RunResolved)
    ));
    assert_eq!(sim.contamination(), contamination);
    assert_eq!(sim.events().len(), events);
}

fn sim_at_contamination(actions: usize) -> Simulator {
    let mut sim = quiet_sim();
    for _ in 0..actions {
        sim.apply(Intervention::AddToxin(0.0)).unwrap();
    }
    sim
}

#[test]
fn scan_metadata_exposes_deterministic_noise_degradation() {
    let mut stable = quiet_sim();
    let stable_event = stable.apply(Intervention::ScanPopulation).unwrap();
    let stable_measurement = &stable_event.measurements[0];
    assert_eq!(
        stable_measurement.contamination_level,
        ContaminationLevel::Stable
    );
    assert_eq!(stable_measurement.contamination_multiplier, 1.0);
    assert_eq!(
        stable_measurement.effective_sigma,
        stable_measurement.base_sigma
    );

    let mut compromised = sim_at_contamination(10);
    let compromised_event = compromised.apply(Intervention::ScanPopulation).unwrap();
    let compromised_measurement = &compromised_event.measurements[0];
    assert_eq!(
        compromised_measurement.contamination_level,
        ContaminationLevel::Compromised
    );
    assert_eq!(compromised_measurement.contamination_multiplier, 1.5);
    assert_eq!(
        compromised_measurement.effective_sigma,
        compromised_measurement.base_sigma * 1.5
    );

    let mut critical = sim_at_contamination(15);
    let critical_event = critical.apply(Intervention::ScanPopulation).unwrap();
    let critical_measurement = &critical_event.measurements[0];
    assert_eq!(
        critical_measurement.contamination_level,
        ContaminationLevel::Critical
    );
    assert_eq!(critical_measurement.contamination_multiplier, 2.25);
    assert_eq!(
        critical_measurement.effective_sigma,
        critical_measurement.base_sigma * 2.25
    );
    assert_eq!(
        stable_measurement.true_value,
        compromised_measurement.true_value
    );
    assert_eq!(
        stable_measurement.true_value,
        critical_measurement.true_value
    );
}

#[test]
fn contaminated_runs_are_deterministic_and_debrief_peak_is_recorded() {
    let mut first = quiet_sim();
    let mut second = quiet_sim();
    let actions = [
        (10, Intervention::AddToxin(0.0)),
        (1, Intervention::ScanPopulation),
        (5, Intervention::AddToxin(0.0)),
        (1, Intervention::ScanChemicals),
        (5, Intervention::AddToxin(0.0)),
    ];
    for (count, action) in actions {
        for _ in 0..count {
            first.apply(action.clone()).unwrap();
            second.apply(action.clone()).unwrap();
        }
    }

    assert_eq!(first.events(), second.events());
    assert_eq!(
        first.run_state().status,
        RunStatus::Failed(RunFailure::ContainmentLost)
    );
    assert_eq!(first.debrief(), second.debrief());
    let debrief = first.debrief().unwrap();
    assert_eq!(debrief.final_contamination, 40.0);
    assert_eq!(debrief.peak_contamination, 40.0);
    assert_eq!(debrief.final_contamination_level, ContaminationLevel::Lost);
    assert_eq!(debrief.compromised_scans, 1);
    assert_eq!(debrief.critical_scans, 1);
}
