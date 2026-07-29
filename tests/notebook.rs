use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use xenolab::engine::interventions::Intervention;
use xenolab::engine::notebook::{
    HypothesisDirection, HypothesisId, NotebookError, ObservableVariable, NOTEBOOK_CAPACITY,
};
use xenolab::engine::run::RunStatus;
use xenolab::engine::runlog::hash_events;
use xenolab::engine::sim::Simulator;
use xenolab::ui::app::App;
use xenolab::worldgen::generate_playable;

fn sim() -> Simulator {
    Simulator::new_no_noise(generate_playable(42))
}

#[test]
fn observable_vocabulary_is_stable_and_maps_to_true_state() {
    assert_eq!(ObservableVariable::ALL.len(), 6);
    assert_eq!(ObservableVariable::ALL[0].label(), "Plant population");
    assert_eq!(ObservableVariable::ALL[1].label(), "Fungus population");
    assert_eq!(ObservableVariable::ALL[2].label(), "Bacteria population");
    assert_eq!(ObservableVariable::ALL[3].label(), "Toxin concentration");
    assert_eq!(ObservableVariable::ALL[4].label(), "Nutrient concentration");
    assert_eq!(ObservableVariable::ALL[5].label(), "UV level");

    let simulator = sim();
    for variable in ObservableVariable::ALL {
        assert_eq!(
            variable.value(simulator.state()),
            simulator.state().get(variable.node())
        );
    }
    assert!(!ObservableVariable::ALL
        .iter()
        .any(|variable| variable.label().contains("Enzyme")));
}

#[test]
fn add_validation_capacity_and_insertion_order_are_engine_owned() {
    let mut simulator = sim();
    let before = (
        simulator.run_state().actions_used,
        simulator.tick_index(),
        simulator.contamination(),
        simulator.events().len(),
    );

    let first = simulator
        .add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    let second = simulator
        .add_hypothesis(
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Decreases,
            ObservableVariable::FungusPopulation,
        )
        .unwrap();
    assert_eq!(first, HypothesisId(1));
    assert_eq!(second, HypothesisId(2));
    assert_eq!(simulator.notebook().hypotheses()[0].id, first);
    assert_eq!(simulator.notebook().hypotheses()[1].id, second);
    assert_eq!(
        simulator.notebook().remaining_slots(),
        NOTEBOOK_CAPACITY - 2
    );
    assert_eq!(before.0, simulator.run_state().actions_used);
    assert_eq!(before.1, simulator.tick_index());
    assert_eq!(before.2, simulator.contamination());
    assert_eq!(before.3, simulator.events().len());

    assert_eq!(
        simulator.add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        ),
        Err(NotebookError::DuplicateHypothesis)
    );
    assert_eq!(
        simulator.add_hypothesis(
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        ),
        Err(NotebookError::SameVariable)
    );

    let pairs = [
        (
            ObservableVariable::ToxinConcentration,
            ObservableVariable::PlantPopulation,
        ),
        (
            ObservableVariable::NutrientConcentration,
            ObservableVariable::PlantPopulation,
        ),
        (
            ObservableVariable::UvLevel,
            ObservableVariable::PlantPopulation,
        ),
        (
            ObservableVariable::FungusPopulation,
            ObservableVariable::BacteriaPopulation,
        ),
        (
            ObservableVariable::BacteriaPopulation,
            ObservableVariable::ToxinConcentration,
        ),
        (
            ObservableVariable::ToxinConcentration,
            ObservableVariable::NutrientConcentration,
        ),
    ];
    for (cause, effect) in pairs {
        simulator
            .add_hypothesis(cause, HypothesisDirection::Increases, effect)
            .unwrap();
    }
    assert_eq!(simulator.notebook().hypotheses().len(), NOTEBOOK_CAPACITY);
    assert_eq!(
        simulator.add_hypothesis(
            ObservableVariable::NutrientConcentration,
            HypothesisDirection::Decreases,
            ObservableVariable::UvLevel,
        ),
        Err(NotebookError::NotebookFull)
    );
}

#[test]
fn edit_and_remove_preserve_ids_positions_and_failed_state() {
    let mut simulator = sim();
    let first = simulator
        .add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    let second = simulator
        .add_hypothesis(
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Decreases,
            ObservableVariable::BacteriaPopulation,
        )
        .unwrap();

    simulator
        .edit_hypothesis(
            first,
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Decreases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    assert_eq!(simulator.notebook().hypotheses()[0].id, first);
    assert_eq!(
        simulator.notebook().hypotheses()[0].direction,
        HypothesisDirection::Decreases
    );

    let snapshot = simulator.notebook().clone();
    assert_eq!(
        simulator.edit_hypothesis(
            first,
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Decreases,
            ObservableVariable::BacteriaPopulation,
        ),
        Err(NotebookError::DuplicateHypothesis)
    );
    assert_eq!(
        simulator.edit_hypothesis(
            first,
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        ),
        Err(NotebookError::SameVariable)
    );
    assert_eq!(*simulator.notebook(), snapshot);
    assert_eq!(
        simulator.edit_hypothesis(
            HypothesisId(999),
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::FungusPopulation,
        ),
        Err(NotebookError::HypothesisNotFound)
    );

    simulator.remove_hypothesis(first).unwrap();
    assert_eq!(simulator.notebook().hypotheses()[0].id, second);
    assert_eq!(
        simulator.remove_hypothesis(first),
        Err(NotebookError::HypothesisNotFound)
    );
}

#[test]
fn notebook_edits_do_not_change_event_hash_or_rng_sequence() {
    let recipe = generate_playable(42);
    let mut plain = Simulator::new(recipe.clone());
    let mut with_notebook = Simulator::new(recipe);
    with_notebook
        .add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    with_notebook
        .edit_hypothesis(
            HypothesisId(1),
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Decreases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    with_notebook.remove_hypothesis(HypothesisId(1)).unwrap();

    let plain_event = plain.apply(Intervention::ScanPopulation).unwrap();
    let notebook_event = with_notebook.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(plain_event, notebook_event);
    assert_eq!(
        hash_events(plain.events()),
        hash_events(with_notebook.events())
    );
}

#[test]
fn resolved_notebook_is_read_only_and_debrief_preserves_snapshot() {
    let mut simulator = sim();
    simulator
        .add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    for _ in 0..30 {
        if simulator.run_state().status != RunStatus::Active {
            break;
        }
        simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_ne!(simulator.run_state().status, RunStatus::Active);
    assert_eq!(
        simulator.add_hypothesis(
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        ),
        Err(NotebookError::RunResolved)
    );
    assert_eq!(
        simulator.remove_hypothesis(HypothesisId(1)),
        Err(NotebookError::RunResolved)
    );
    assert_eq!(
        simulator.debrief().unwrap().notebook,
        simulator.notebook().hypotheses()
    );
}

#[test]
fn app_restart_clears_notebook() {
    let mut app = App::new(42);
    app.simulator
        .add_hypothesis(
            ObservableVariable::FungusPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.active_view.as_index(), 3);
    for _ in 0..30 {
        if app.simulator.run_state().status != RunStatus::Active {
            break;
        }
        app.simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.simulator.notebook().hypotheses().is_empty());
}
