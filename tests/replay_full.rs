use xenolab::engine::interventions::Intervention;
use xenolab::engine::notebook::{HypothesisDirection, HypothesisId, ObservableVariable};
use xenolab::engine::repair::RepairTrack;
use xenolab::engine::replay::{replay, HypothesisDraft, ReplayError, ReplayOperation};
use xenolab::engine::run::{RunFailure, RunStatus};
use xenolab::engine::runlog::hash_events;
use xenolab::worldgen::generate_playable;

fn nutrient_draft() -> HypothesisDraft {
    HypothesisDraft {
        cause: ObservableVariable::NutrientConcentration,
        direction: HypothesisDirection::Increases,
        effect: ObservableVariable::PlantPopulation,
    }
}

fn complete_research_operations() -> Vec<ReplayOperation> {
    let mut operations = vec![
        ReplayOperation::NotebookAdd(nutrient_draft()),
        ReplayOperation::NotebookAdd(HypothesisDraft {
            cause: ObservableVariable::ToxinConcentration,
            direction: HypothesisDirection::Decreases,
            effect: ObservableVariable::BacteriaPopulation,
        }),
        ReplayOperation::NotebookEdit {
            id: HypothesisId(2),
            replacement: HypothesisDraft {
                cause: ObservableVariable::ToxinConcentration,
                direction: HypothesisDirection::Increases,
                effect: ObservableVariable::BacteriaPopulation,
            },
        },
        ReplayOperation::NotebookEdit {
            id: HypothesisId(2),
            replacement: HypothesisDraft {
                cause: ObservableVariable::ToxinConcentration,
                direction: HypothesisDirection::Decreases,
                effect: ObservableVariable::BacteriaPopulation,
            },
        },
        ReplayOperation::NotebookRemove(HypothesisId(2)),
    ];
    for _ in 0..3 {
        operations.push(ReplayOperation::Apply(Intervention::AddNutrient(20.0)));
        operations.push(ReplayOperation::Apply(Intervention::ScanPopulation));
    }
    operations.push(ReplayOperation::Publish(HypothesisId(1)));
    operations.push(ReplayOperation::PurchaseRepair(RepairTrack::Calibration));
    operations
}

#[test]
fn complete_research_run_replays_all_outcome_relevant_state() {
    let operations = complete_research_operations();
    let original = replay(21, &operations).unwrap();
    let repeated = replay(21, &operations).unwrap();

    assert_eq!(generate_playable(21), generate_playable(21));
    assert_eq!(*original.state(), *repeated.state());
    assert_eq!(original.run_state(), repeated.run_state());
    assert_eq!(original.contamination(), repeated.contamination());
    assert_eq!(original.notebook(), repeated.notebook());
    assert_eq!(original.publications(), repeated.publications());
    assert_eq!(original.credits_earned(), repeated.credits_earned());
    assert_eq!(original.credits_spent(), repeated.credits_spent());
    assert_eq!(original.credits_available(), repeated.credits_available());
    assert_eq!(original.calibration_level(), repeated.calibration_level());
    assert_eq!(original.containment_level(), repeated.containment_level());
    assert_eq!(original.repair_purchases(), repeated.repair_purchases());
    assert_eq!(original.events(), repeated.events());
    assert_eq!(original.debrief(), repeated.debrief());
    assert_eq!(
        hash_events(original.events()),
        hash_events(repeated.events())
    );
    assert_eq!(original.verification_hash(), repeated.verification_hash());
    assert!(original.credits_earned() > 0);
}

#[test]
fn successful_budget_and_containment_runs_replay_deterministically() {
    let success = (0..20)
        .map(|_| ReplayOperation::Apply(Intervention::SetUvHigh))
        .collect::<Vec<_>>();
    let successful = replay(12, &success).unwrap();
    assert_eq!(successful.run_state().status, RunStatus::Won);

    let budget = (0..30)
        .map(|_| ReplayOperation::Apply(Intervention::ScanChemicals))
        .collect::<Vec<_>>();
    let budget_run = replay(12, &budget).unwrap();
    assert_eq!(
        budget_run.run_state().status,
        RunStatus::Failed(RunFailure::ActionBudgetExhausted)
    );

    let containment = (0..14)
        .map(|_| ReplayOperation::Apply(Intervention::SterilizeSample))
        .collect::<Vec<_>>();
    let containment_run = replay(12, &containment).unwrap();
    assert_eq!(
        containment_run.run_state().status,
        RunStatus::Failed(RunFailure::ContainmentLost)
    );
    let containment_repeat = replay(12, &containment).unwrap();
    assert_eq!(containment_run.state(), containment_repeat.state());
    assert_eq!(containment_run.events(), containment_repeat.events());
    assert_eq!(
        containment_run.verification_hash(),
        containment_repeat.verification_hash()
    );
}

#[test]
fn verification_hash_is_sensitive_to_non_gameplay_history() {
    let base = replay(21, &[]).unwrap();
    let with_notebook = replay(21, &[ReplayOperation::NotebookAdd(nutrient_draft())]).unwrap();
    let with_research = replay(21, &complete_research_operations()).unwrap();
    assert_ne!(base.verification_hash(), with_notebook.verification_hash());
    assert_ne!(base.verification_hash(), with_research.verification_hash());
    assert_eq!(
        hash_events(base.events()),
        hash_events(with_notebook.events())
    );
}

#[test]
fn invalid_replay_operations_fail_with_operation_context() {
    let error = match replay(21, &[ReplayOperation::NotebookRemove(HypothesisId(999))]) {
        Ok(_) => panic!("invalid replay unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(matches!(error, ReplayError::Notebook { index: 0, .. }));
}

#[test]
fn replay_restart_operations_clear_run_local_state() {
    let operations = vec![
        ReplayOperation::Apply(Intervention::AddToxin(20.0)),
        ReplayOperation::NotebookAdd(nutrient_draft()),
        ReplayOperation::RestartSameSeed,
        ReplayOperation::RestartNewSeed(10),
    ];
    let restarted = replay(21, &operations).unwrap();
    assert_eq!(restarted.run_state().status, RunStatus::Active);
    assert_eq!(restarted.run_state().seed, 10);
    assert_eq!(restarted.run_state().actions_used, 0);
    assert_eq!(restarted.contamination(), 0.0);
    assert!(restarted.notebook().hypotheses().is_empty());
}
