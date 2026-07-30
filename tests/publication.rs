use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::node::EdgeSpec;
use xenolab::engine::notebook::{
    HypothesisDirection, HypothesisId, NotebookError, ObservableVariable,
};
use xenolab::engine::publication::{
    EvidenceStrength, PublicationError, PublicationRationale, MAX_RESEARCH_CREDITS,
    PUBLICATION_LIMIT,
};
use xenolab::engine::run::{RunFailure, RunStatus};
use xenolab::engine::runlog::hash_events;
use xenolab::engine::sim::Simulator;
use xenolab::worldgen::generate_playable;

fn toxin_bacteria_sim() -> Simulator {
    let mut recipe = xenolab::engine::world::WorldRecipe::placeholder(42, 0);
    recipe.edges = vec![EdgeSpec {
        from: NodeId::Toxin,
        to: NodeId::BacteriaPop,
        weight: -1.0,
    }];
    Simulator::new_no_noise(recipe)
}

fn add_claim(simulator: &mut Simulator) -> HypothesisId {
    simulator
        .add_hypothesis(
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Decreases,
            ObservableVariable::BacteriaPopulation,
        )
        .unwrap()
}

fn add_trial(simulator: &mut Simulator, delta: f32) {
    simulator.apply(Intervention::AddToxin(delta)).unwrap();
    simulator.apply(Intervention::ScanPopulation).unwrap();
}

fn nutrient_plant_sim() -> Simulator {
    let mut recipe = xenolab::engine::world::WorldRecipe::placeholder(42, 0);
    recipe.edges = vec![EdgeSpec {
        from: NodeId::Nutrient,
        to: NodeId::PlantPop,
        weight: 1.0,
    }];
    Simulator::new_no_noise(recipe)
}

fn add_nutrient_trial(simulator: &mut Simulator) {
    simulator.apply(Intervention::AddNutrient(5.0)).unwrap();
    simulator.apply(Intervention::ScanPopulation).unwrap();
}

#[test]
fn publication_requires_relevant_observed_evidence() {
    let mut no_observation = toxin_bacteria_sim();
    let id = add_claim(&mut no_observation);
    no_observation.apply(Intervention::AddToxin(5.0)).unwrap();
    let publication = no_observation.publish_hypothesis(id).unwrap();
    assert_eq!(publication.evidence_strength, EvidenceStrength::Unsupported);
    assert_eq!(publication.credits_awarded, 0);
    assert_eq!(
        publication.evidence_summary.rationale,
        PublicationRationale::NoRelevantEvidence
    );

    let mut observed = toxin_bacteria_sim();
    let id = add_claim(&mut observed);
    add_trial(&mut observed, 5.0);
    let publication = observed.publish_hypothesis(id).unwrap();
    assert_eq!(publication.evidence_summary.relevant_trials, 1);
    assert_eq!(publication.evidence_summary.supporting_trials, 1);
    assert_eq!(publication.evidence_strength, EvidenceStrength::Weak);
    assert_eq!(publication.credits_awarded, 1);
}

#[test]
fn wrong_direction_is_structurally_rejected_without_hidden_details() {
    let mut simulator = toxin_bacteria_sim();
    let id = simulator
        .add_hypothesis(
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::BacteriaPopulation,
        )
        .unwrap();
    add_trial(&mut simulator, 5.0);
    let publication = simulator.publish_hypothesis(id).unwrap();
    assert_eq!(publication.evidence_strength, EvidenceStrength::Unsupported);
    assert_eq!(publication.credits_awarded, 0);
    assert_eq!(
        publication.evidence_summary.rationale,
        PublicationRationale::EvidenceContradictsClaim
    );
}

#[test]
fn moderate_and_strong_grades_follow_replication_policy() {
    let mut moderate = toxin_bacteria_sim();
    let moderate_id = add_claim(&mut moderate);
    add_trial(&mut moderate, 5.0);
    add_trial(&mut moderate, 5.0);
    let publication = moderate.publish_hypothesis(moderate_id).unwrap();
    assert_eq!(publication.evidence_strength, EvidenceStrength::Moderate);
    assert_eq!(publication.credits_awarded, 2);
    assert_eq!(publication.evidence_summary.supporting_trials, 2);

    let mut strong = nutrient_plant_sim();
    let strong_id = strong
        .add_hypothesis(
            ObservableVariable::NutrientConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    add_nutrient_trial(&mut strong);
    add_nutrient_trial(&mut strong);
    add_nutrient_trial(&mut strong);
    let publication = strong.publish_hypothesis(strong_id).unwrap();
    assert_eq!(publication.evidence_strength, EvidenceStrength::Strong);
    assert_eq!(publication.credits_awarded, 3);
    assert_eq!(publication.evidence_summary.supporting_trials, 3);
    assert_eq!(publication.evidence_summary.contradicting_trials, 0);
    assert_eq!(
        publication
            .evidence_summary
            .distinct_intervention_directions,
        1
    );
}

#[test]
fn publication_costs_one_action_without_simulation_changes_or_event_hash_changes() {
    let mut simulator = toxin_bacteria_sim();
    let id = add_claim(&mut simulator);
    let before_state = *simulator.state();
    let before_tick = simulator.tick_index();
    let before_contamination = simulator.contamination();
    let before_events = simulator.events().len();
    let before_hash = hash_events(simulator.events());
    let publication = simulator.publish_hypothesis(id).unwrap();
    assert_eq!(publication.action_number, 1);
    assert_eq!(simulator.run_state().actions_used, 1);
    assert_eq!(*simulator.state(), before_state);
    assert_eq!(simulator.tick_index(), before_tick);
    assert_eq!(simulator.contamination(), before_contamination);
    assert_eq!(simulator.events().len(), before_events);
    assert_eq!(hash_events(simulator.events()), before_hash);
}

#[test]
fn novelty_limit_and_published_immutability_are_enforced() {
    let mut simulator = toxin_bacteria_sim();
    let id = add_claim(&mut simulator);
    simulator.publish_hypothesis(id).unwrap();
    assert_eq!(
        simulator.publish_hypothesis(id),
        Err(PublicationError::HypothesisAlreadyPublished)
    );
    assert_eq!(
        simulator.edit_hypothesis(
            id,
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::BacteriaPopulation,
        ),
        Err(NotebookError::HypothesisAlreadyPublished)
    );
    assert_eq!(
        simulator.remove_hypothesis(id),
        Err(NotebookError::HypothesisAlreadyPublished)
    );

    for (cause, effect) in [
        (
            ObservableVariable::ToxinConcentration,
            ObservableVariable::PlantPopulation,
        ),
        (
            ObservableVariable::ToxinConcentration,
            ObservableVariable::FungusPopulation,
        ),
        (
            ObservableVariable::ToxinConcentration,
            ObservableVariable::NutrientConcentration,
        ),
    ] {
        let hypothesis_id = simulator
            .add_hypothesis(cause, HypothesisDirection::Increases, effect)
            .unwrap();
        simulator.publish_hypothesis(hypothesis_id).unwrap();
    }
    assert_eq!(simulator.publications().len() as u32, PUBLICATION_LIMIT);
    assert_eq!(simulator.research_credits(), 0);
    assert_eq!(simulator.max_research_credits(), MAX_RESEARCH_CREDITS);
    let extra = simulator
        .add_hypothesis(
            ObservableVariable::NutrientConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::BacteriaPopulation,
        )
        .unwrap();
    assert_eq!(
        simulator.publish_hypothesis(extra),
        Err(PublicationError::PublicationLimitReached)
    );
}

#[test]
fn failed_publication_does_not_consume_action_and_resolved_runs_lock_publication() {
    let mut simulator = Simulator::new_no_noise(generate_playable(42));
    assert_eq!(
        simulator.publish_hypothesis(HypothesisId(999)),
        Err(PublicationError::HypothesisNotFound)
    );
    assert_eq!(simulator.run_state().actions_used, 0);

    let id = simulator
        .add_hypothesis(
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::FungusPopulation,
        )
        .unwrap();
    for _ in 0..30 {
        if simulator.run_state().status != RunStatus::Active {
            break;
        }
        simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_eq!(
        simulator.publish_hypothesis(id),
        Err(PublicationError::RunResolved)
    );
    assert!(matches!(
        simulator.run_state().status,
        RunStatus::Failed(RunFailure::ActionBudgetExhausted) | RunStatus::Won
    ));
}

#[test]
fn final_action_publication_resolves_budget_and_restart_clears_publication_state() {
    let mut simulator = Simulator::new_no_noise(generate_playable(42));
    let id = simulator
        .add_hypothesis(
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::FungusPopulation,
        )
        .unwrap();
    for _ in 0..29 {
        simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    let publication = simulator.publish_hypothesis(id).unwrap();
    assert_eq!(publication.action_number, 30);
    assert_eq!(
        simulator.run_state().status,
        RunStatus::Failed(RunFailure::ActionBudgetExhausted)
    );
    assert_eq!(simulator.debrief().unwrap().publications.len(), 1);

    let mut app = xenolab::ui::app::App::new(42);
    let id = app
        .simulator
        .add_hypothesis(
            ObservableVariable::PlantPopulation,
            HypothesisDirection::Increases,
            ObservableVariable::FungusPopulation,
        )
        .unwrap();
    app.simulator.publish_hypothesis(id).unwrap();
    for _ in 0..29 {
        app.simulator.apply(Intervention::ScanPopulation).unwrap();
    }
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.simulator.publications().is_empty());
    assert_eq!(app.simulator.research_credits(), 0);
    assert!(app.simulator.notebook().hypotheses().is_empty());
}

#[test]
fn publication_determinism_and_restart_state_are_run_local() {
    let mut first = toxin_bacteria_sim();
    let id = add_claim(&mut first);
    add_trial(&mut first, 5.0);
    let first_publication = first.publish_hypothesis(id).unwrap();

    let mut second = toxin_bacteria_sim();
    let id = add_claim(&mut second);
    add_trial(&mut second, 5.0);
    let second_publication = second.publish_hypothesis(id).unwrap();
    assert_eq!(first_publication, second_publication);

    let mut app = xenolab::ui::app::App::new(42);
    let id = app
        .simulator
        .add_hypothesis(
            ObservableVariable::ToxinConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::BacteriaPopulation,
        )
        .unwrap();
    app.simulator.publish_hypothesis(id).unwrap();
    assert_eq!(app.simulator.publications().len(), 1);
    assert_eq!(app.simulator.research_credits(), 0);
}
