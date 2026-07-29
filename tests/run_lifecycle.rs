use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pretty_assertions::assert_eq;
use xenolab::engine::ids::{NodeId, NODE_COUNT};
use xenolab::engine::interventions::Intervention;
use xenolab::engine::run::{RunFailure, RunStatus, ACTION_LIMIT};
use xenolab::engine::runlog::hash_events;
use xenolab::engine::sim::{SimError, Simulator};
use xenolab::engine::world::WorldState;
use xenolab::ui::app::App;
use xenolab::worldgen;

fn recipe_with_state(seed: u64, state: WorldState) -> xenolab::engine::world::WorldRecipe {
    let mut recipe = worldgen::generate_playable(seed);
    recipe.initial_state = state;
    recipe.noise_sigma = [0.0; NODE_COUNT];
    recipe
}

fn state(plant: f32, fungus: f32, bacteria: f32, toxin: f32) -> WorldState {
    WorldState {
        values: [50.0, plant, fungus, bacteria, toxin, 50.0, 50.0],
    }
}

fn scan_for_objective(sim: &mut Simulator, seed: u64) {
    let action = match seed % 3 {
        1 => Intervention::ScanChemicals,
        _ => Intervention::ScanPopulation,
    };
    sim.apply(action).unwrap();
}

#[test]
fn each_objective_tracks_progress_and_wins() {
    for (seed, initial) in [
        (3, state(60.0, 20.0, 20.0, 30.0)),
        (4, state(40.0, 20.0, 20.0, 15.0)),
        (5, state(25.0, 20.0, 25.0, 30.0)),
    ] {
        let mut sim = Simulator::new(recipe_with_state(seed, initial));
        assert_eq!(sim.run_state().objective_progress.current, 0);
        assert_eq!(sim.run_state().status, RunStatus::Active);

        scan_for_objective(&mut sim, seed);
        assert_eq!(sim.run_state().objective_progress.current, 1);
        scan_for_objective(&mut sim, seed);
        assert_eq!(sim.run_state().objective_progress.current, 2);
        scan_for_objective(&mut sim, seed);

        assert_eq!(sim.run_state().objective_progress.current, 3);
        assert_eq!(sim.run_state().status, RunStatus::Won);
        assert_eq!(sim.run_state().actions_used, 3);
        assert!(sim.debrief().is_some());
    }
}

#[test]
fn non_qualifying_state_resets_progress_and_measurements_do_not_drive_it() {
    let mut qualifying = Simulator::new(recipe_with_state(3, state(60.0, 20.0, 20.0, 30.0)));
    qualifying.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(qualifying.run_state().objective_progress.current, 1);

    let mut non_qualifying = Simulator::new(recipe_with_state(3, state(59.9, 20.0, 20.0, 30.0)));
    non_qualifying.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(non_qualifying.run_state().objective_progress.current, 0);
}

#[test]
fn actions_consume_budget_but_scans_do_not_advance_time() {
    let mut sim = Simulator::new(recipe_with_state(3, state(10.0, 20.0, 20.0, 30.0)));
    sim.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(sim.run_state().actions_used, 1);
    assert_eq!(sim.run_state().actions_remaining(), ACTION_LIMIT - 1);
    assert_eq!(sim.tick_index(), 0);

    sim.apply(Intervention::AdvanceTime).unwrap();
    assert_eq!(sim.run_state().actions_used, 2);
    assert_eq!(sim.tick_index(), 1);

    let before = sim.run_state().actions_used;
    assert!(matches!(
        sim.apply(Intervention::AddNutrient(-1.0)),
        Err(SimError::InvalidDelta("AddNutrient"))
    ));
    assert_eq!(sim.run_state().actions_used, before);
}

#[test]
fn budget_failure_and_terminal_lockout_are_deterministic() {
    let mut sim = Simulator::new(recipe_with_state(3, state(10.0, 20.0, 20.0, 30.0)));
    for _ in 0..(ACTION_LIMIT - 1) {
        sim.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_eq!(sim.run_state().status, RunStatus::Active);

    sim.apply(Intervention::ScanPopulation).unwrap();
    let debrief = sim.debrief().unwrap().clone();
    let state_before = *sim.state();
    let tick_before = sim.tick_index();
    let used_before = sim.run_state().actions_used;
    let events_before = sim.events().len();
    let hash_before = hash_events(sim.events());

    assert_eq!(
        sim.run_state().status,
        RunStatus::Failed(RunFailure::ActionBudgetExhausted)
    );
    assert_eq!(
        debrief.failure_reason,
        Some(RunFailure::ActionBudgetExhausted)
    );
    assert_eq!(debrief.event_hash, hash_before.to_hex().to_string());
    assert!(matches!(
        sim.apply(Intervention::AdvanceTime),
        Err(SimError::RunResolved)
    ));
    assert_eq!(*sim.state(), state_before);
    assert_eq!(sim.tick_index(), tick_before);
    assert_eq!(sim.run_state().actions_used, used_before);
    assert_eq!(sim.events().len(), events_before);
    assert_eq!(hash_events(sim.events()), hash_before);
}

#[test]
fn final_action_win_precedes_budget_failure() {
    let mut recipe = recipe_with_state(3, state(59.0, 20.0, 20.0, 30.0));
    recipe.edges.clear();
    recipe.biases = [0.0; NODE_COUNT];
    recipe.biases[NodeId::PlantPop.as_index()] = 3.0;
    let mut sim = Simulator::new_no_noise(recipe);

    for _ in 0..27 {
        sim.apply(Intervention::ScanPopulation).unwrap();
    }
    sim.apply(Intervention::SetUvLow).unwrap();
    sim.apply(Intervention::ScanPopulation).unwrap();
    sim.apply(Intervention::ScanPopulation).unwrap();

    assert_eq!(sim.run_state().actions_used, ACTION_LIMIT);
    assert_eq!(sim.run_state().status, RunStatus::Won);
}

#[test]
fn equal_runs_have_equal_debriefs() {
    let recipe = recipe_with_state(3, state(60.0, 20.0, 20.0, 30.0));
    let mut first = Simulator::new(recipe.clone());
    let mut second = Simulator::new(recipe);
    for _ in 0..3 {
        first.apply(Intervention::ScanPopulation).unwrap();
        second.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_eq!(first.debrief(), second.debrief());
    assert_eq!(
        first.debrief().unwrap().event_hash,
        hash_events(first.events()).to_hex().to_string()
    );
}

#[test]
fn app_restart_controls_reset_same_and_new_seed_runs() {
    let mut app = App::new(42);
    for _ in 0..30 {
        app.simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    assert!(app.is_resolved());
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.seed, 42);
    assert!(!app.is_resolved());
    assert_eq!(app.simulator.run_state().actions_used, 0);
    assert!(app.simulator.events().is_empty());

    for _ in 0..30 {
        app.simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .unwrap();
    for ch in "123".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.seed, 123);
    assert!(!app.is_resolved());
    assert_eq!(app.simulator.run_state().actions_used, 0);
}
