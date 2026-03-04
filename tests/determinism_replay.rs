use pretty_assertions::assert_eq;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::runlog::{hash_events, RunEvent};
use xenolab::engine::sim::Simulator;
use xenolab::engine::world::WorldState;
use xenolab::worldgen;

fn run_script(seed: u64) -> (Vec<RunEvent>, WorldState) {
    let recipe = worldgen::generate_playable(seed);
    let mut sim = Simulator::new(recipe);

    let script = [
        Intervention::ScanPopulation,
        Intervention::SetUvHigh,
        Intervention::AdvanceTime,
        Intervention::AddNutrient(20.0),
        Intervention::AdvanceTime,
        Intervention::AddToxin(20.0),
        Intervention::AdvanceTime,
        Intervention::NeutralizeToxin(20.0),
        Intervention::AdvanceTime,
        Intervention::RemoveFungus,
        Intervention::AdvanceTime,
    ];

    let mut events = Vec::new();
    for action in script {
        let event = sim.apply(action).unwrap();
        events.push(event);
    }

    (events, *sim.state())
}

#[test]
fn replay_hash_matches() {
    let (events_a, state_a) = run_script(42);
    let (events_b, state_b) = run_script(42);

    assert_eq!(state_a, state_b, "final state mismatch for replay");
    assert_eq!(
        hash_events(&events_a),
        hash_events(&events_b),
        "runlog hash mismatch for replay"
    );
}
