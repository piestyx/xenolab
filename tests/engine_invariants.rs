use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::invariants::{assert_state_in_bounds, state_delta};
use xenolab::engine::sim::Simulator;
use xenolab::worldgen;

fn run_ticks(sim: &mut Simulator, count: usize) {
    for _ in 0..count {
        sim.apply(Intervention::AdvanceTime).unwrap();
    }
}

#[test]
fn clamp_and_finite_invariant() {
    let action_cycle = [
        Intervention::SetUvHigh,
        Intervention::AddNutrient(20.0),
        Intervention::AddToxin(20.0),
        Intervention::NeutralizeToxin(20.0),
        Intervention::RemoveFungus,
        Intervention::RemoveBacteria,
        Intervention::ScanPopulation,
        Intervention::ScanChemicals,
    ];

    for seed in 1_u64..=20 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe);

        for step in 0..20 {
            let action = action_cycle[step % action_cycle.len()].clone();
            sim.apply(action).unwrap();
            assert_state_in_bounds(sim.state())
                .unwrap_or_else(|err| panic!("seed {seed} step {step}: {err}"));
        }
    }
}

#[test]
fn uv_set_is_exact() {
    let recipe = worldgen::generate_playable(1);
    let mut sim = Simulator::new_no_noise_for_analysis(recipe);

    sim.apply(Intervention::SetUvLow).unwrap();
    assert_eq!(sim.state().get(NodeId::UvLevel), 0.0);

    sim.apply(Intervention::SetUvHigh).unwrap();
    assert_eq!(sim.state().get(NodeId::UvLevel), 100.0);

    sim.apply(Intervention::ScanPopulation).unwrap();
    assert_eq!(sim.state().get(NodeId::UvLevel), 100.0);
}

#[test]
fn neutralize_toxin_never_increases() {
    let recipe = worldgen::generate_playable(2);
    let mut sim = Simulator::new_no_noise_for_analysis(recipe);

    for i in 0..5 {
        sim.apply(Intervention::AddToxin(20.0)).unwrap();
        let before = sim.state().get(NodeId::Toxin);
        sim.apply(Intervention::NeutralizeToxin(20.0)).unwrap();
        let after = sim.state().get(NodeId::Toxin);

        assert!(
            after <= before,
            "toxin increased after neutralization at iteration {i}: before={before} after={after}"
        );
    }
}

#[test]
fn remove_fungus_behavior_matches_topology() {
    let recipe = worldgen::generate_playable(3);
    let can_regrow =
        recipe.metadata.has_twist_toxin_fungus || recipe.metadata.has_twist_nutrient_fungus;
    let mut sim = Simulator::new_no_noise_for_analysis(recipe.clone());

    sim.apply(Intervention::RemoveFungus).unwrap();

    let actions = [
        Intervention::SetUvHigh,
        Intervention::ScanPopulation,
        Intervention::SetUvLow,
        Intervention::ScanChemicals,
        Intervention::AdvanceTime,
    ];

    for (idx, action) in actions.into_iter().enumerate() {
        sim.apply(action).unwrap();
        let fungus = sim.state().get(NodeId::FungusLoad);
        if can_regrow {
            assert!(
                fungus <= 5.0,
                "fungus regrowth exceeded low-band with twist at step {idx}: {fungus}"
            );
        } else {
            assert_eq!(fungus, 0.0, "fungus should stay zero at step {idx}");
        }
    }

    let start = recipe.initial_state;
    let deltas = state_delta(&start, sim.state());
    assert_eq!(deltas.len(), NodeId::ALL.len());
}

#[test]
fn mandatory_edge_sign_sanity() {
    let mut pass_count = 0_u32;
    let mut failed = Vec::new();

    for seed in 10_u64..=20 {
        let recipe = worldgen::generate_playable(seed);

        let mut toxin_branch = Simulator::new_no_noise_for_analysis(recipe.clone());
        run_ticks(&mut toxin_branch, 3);
        let bacteria_before = toxin_branch.state().get(NodeId::BacteriaPop);
        toxin_branch.apply(Intervention::AddToxin(20.0)).unwrap();
        run_ticks(&mut toxin_branch, 3);
        let bacteria_delta = toxin_branch.state().get(NodeId::BacteriaPop) - bacteria_before;

        let mut uv_branch = Simulator::new_no_noise_for_analysis(recipe);
        run_ticks(&mut uv_branch, 3);
        let plant_before = uv_branch.state().get(NodeId::PlantPop);
        uv_branch.apply(Intervention::SetUvHigh).unwrap();
        run_ticks(&mut uv_branch, 3);
        let plant_delta = uv_branch.state().get(NodeId::PlantPop) - plant_before;

        if bacteria_delta <= -1.0 && plant_delta >= 0.0 {
            pass_count += 1;
        } else {
            failed.push(format!(
                "seed {seed}: bacteria_delta={bacteria_delta:.3}, plant_delta={plant_delta:.3}"
            ));
        }
    }

    assert!(
        pass_count >= 8,
        "mandatory edge sanity below threshold: pass_count={pass_count}, failures={:?}",
        failed
    );
}
