use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::node::EdgeSpec;
use xenolab::engine::notebook::{HypothesisDirection, ObservableVariable};
use xenolab::engine::repair::{CalibrationLevel, ContainmentLevel, RepairError, RepairTrack};
use xenolab::engine::run::{RunFailure, RunStatus};
use xenolab::engine::sim::Simulator;
use xenolab::engine::world::WorldRecipe;

fn credited_sim() -> Simulator {
    let mut recipe = WorldRecipe::placeholder(42, 0);
    recipe.edges = vec![
        EdgeSpec {
            from: NodeId::Nutrient,
            to: NodeId::PlantPop,
            weight: 1.0,
        },
        EdgeSpec {
            from: NodeId::Nutrient,
            to: NodeId::FungusLoad,
            weight: 1.0,
        },
    ];
    let mut sim = Simulator::new_no_noise_for_analysis(recipe);
    let nutrient = sim
        .add_hypothesis(
            ObservableVariable::NutrientConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::PlantPopulation,
        )
        .unwrap();
    for _ in 0..3 {
        sim.apply(Intervention::AddNutrient(5.0)).unwrap();
        sim.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_eq!(sim.publish_hypothesis(nutrient).unwrap().credits_awarded, 3);
    let fungus = sim
        .add_hypothesis(
            ObservableVariable::NutrientConcentration,
            HypothesisDirection::Increases,
            ObservableVariable::FungusPopulation,
        )
        .unwrap();
    for _ in 0..3 {
        sim.apply(Intervention::AddNutrient(5.0)).unwrap();
        sim.apply(Intervention::ScanPopulation).unwrap();
    }
    assert_eq!(sim.publish_hypothesis(fungus).unwrap().credits_awarded, 3);
    sim
}

#[test]
fn repair_levels_have_central_costs_and_effects() {
    assert_eq!(CalibrationLevel::Level0.next_cost(), Some(2));
    assert_eq!(CalibrationLevel::Level1.next_cost(), Some(4));
    assert_eq!(CalibrationLevel::Level2.next_cost(), None);
    assert_eq!(CalibrationLevel::Level0.noise_multiplier(), 1.0);
    assert_eq!(CalibrationLevel::Level1.noise_multiplier(), 0.8);
    assert_eq!(CalibrationLevel::Level2.noise_multiplier(), 0.6);
    assert_eq!(ContainmentLevel::Level0.contamination_reduction(), 0);
    assert_eq!(ContainmentLevel::Level1.contamination_reduction(), 1);
    assert_eq!(ContainmentLevel::Level2.contamination_reduction(), 2);
}

#[test]
fn purchases_spend_wallet_without_gameplay_side_effects() {
    let mut sim = credited_sim();
    let before = (
        *sim.state(),
        sim.tick_index(),
        sim.contamination(),
        sim.events().len(),
    );
    let purchase = sim.purchase_repair(RepairTrack::Calibration).unwrap();
    assert_eq!(purchase.level_before, 0);
    assert_eq!(purchase.level_after, 1);
    assert_eq!(purchase.credits_spent, 2);
    assert_eq!(sim.credits_earned(), 6);
    assert_eq!(sim.credits_spent(), 2);
    assert_eq!(sim.credits_available(), 4);
    assert_eq!(
        (
            *sim.state(),
            sim.tick_index(),
            sim.contamination(),
            sim.events().len()
        ),
        before
    );
    assert_eq!(
        sim.purchase_repair(RepairTrack::Calibration)
            .unwrap()
            .credits_spent,
        4
    );
    assert_eq!(sim.calibration_level(), CalibrationLevel::Level2);
    assert_eq!(
        sim.purchase_repair(RepairTrack::Calibration),
        Err(RepairError::MaximumLevelReached)
    );
}

#[test]
fn containment_reduces_only_future_costs_and_preserves_metadata() {
    let mut sim = credited_sim();
    sim.purchase_repair(RepairTrack::Containment).unwrap();
    assert_eq!(
        sim.effective_contamination_cost(&Intervention::AddToxin(1.0)),
        1
    );
    assert_eq!(
        sim.effective_contamination_cost(&Intervention::SterilizeSample),
        2
    );
    assert_eq!(
        sim.effective_contamination_cost(&Intervention::ScanPopulation),
        0
    );
    let event = sim.apply(Intervention::AddToxin(1.0)).unwrap();
    assert_eq!(event.base_contamination_cost, 2);
    assert_eq!(event.containment_reduction, 1);
    assert_eq!(event.effective_contamination_cost, 1);
}

#[test]
fn calibration_composes_with_contamination_for_future_scans() {
    let mut sim = credited_sim();
    sim.purchase_repair(RepairTrack::Calibration).unwrap();
    sim.apply(Intervention::AddToxin(1.0)).unwrap();
    let event = sim.apply(Intervention::ScanPopulation).unwrap();
    let measurement = &event.measurements[0];
    assert_eq!(measurement.calibration_multiplier, 0.8);
    assert_eq!(measurement.contamination_multiplier, 1.0);
    assert_eq!(measurement.total_multiplier, 0.8);
}

#[test]
fn restart_and_terminal_lockout_clear_or_preserve_repairs() {
    let mut sim = credited_sim();
    sim.purchase_repair(RepairTrack::Calibration).unwrap();
    let restarted = Simulator::new(WorldRecipe::placeholder(42, 0));
    assert_eq!(restarted.credits_available(), 0);
    assert_eq!(restarted.calibration_level(), CalibrationLevel::Level0);
    assert!(restarted.repair_purchases().is_empty());

    let mut failed = Simulator::new(WorldRecipe::placeholder(7, 0));
    for _ in 0..14 {
        failed.apply(Intervention::SterilizeSample).unwrap();
    }
    assert_eq!(
        failed.run_state().status,
        RunStatus::Failed(RunFailure::ContainmentLost)
    );
    assert_eq!(
        failed.purchase_repair(RepairTrack::Calibration),
        Err(RepairError::RunResolved)
    );
}
