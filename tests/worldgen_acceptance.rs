use std::collections::BTreeSet;

use pretty_assertions::{assert_eq, assert_ne};
use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::sim::Simulator;
use xenolab::worldgen;

fn run_ticks(sim: &mut Simulator, count: usize) {
    for _ in 0..count {
        sim.apply(Intervention::AdvanceTime).unwrap();
    }
}

fn branch_plant_delta(seed: u64, interventions: &[Intervention]) -> f32 {
    let recipe = worldgen::generate_playable(seed);
    let mut sim = Simulator::new_no_noise(recipe);
    run_ticks(&mut sim, 3);
    let before = sim.state().get(NodeId::PlantPop);
    for action in interventions {
        sim.apply(action.clone()).unwrap();
    }
    run_ticks(&mut sim, 3);
    sim.state().get(NodeId::PlantPop) - before
}

#[test]
fn determinism_world_recipe() {
    for seed in [1_u64, 2, 3, 10, 999] {
        let a = worldgen::generate_playable(seed);
        let b = worldgen::generate_playable(seed);
        assert_eq!(a, b, "recipe mismatch for seed {seed}");

        let hash_a = a.recipe_hash().unwrap();
        let hash_b = b.recipe_hash().unwrap();
        assert_eq!(hash_a, hash_b, "recipe hash mismatch for seed {seed}");
    }
}

#[test]
fn stability_no_intervention() {
    for seed in 1_u64..=20 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new(recipe);
        let start = *sim.state();
        run_ticks(&mut sim, 30);

        for value in sim.state().values {
            assert!(
                (0.0..=100.0).contains(&value),
                "out of bounds value {value} for seed {seed}"
            );
        }

        let plant_delta = (sim.state().get(NodeId::PlantPop) - start.get(NodeId::PlantPop)).abs();
        let toxin_delta = (sim.state().get(NodeId::Toxin) - start.get(NodeId::Toxin)).abs();
        let bacteria_delta = (sim.state().get(NodeId::BacteriaPop) - start.get(NodeId::BacteriaPop)).abs();
        assert!(
            plant_delta >= 5.0 || toxin_delta >= 5.0 || bacteria_delta >= 5.0,
            "dead-flat behavior for seed {seed}: plant={plant_delta}, toxin={toxin_delta}, bacteria={bacteria_delta}"
        );
    }
}

#[test]
fn signature_uv_affects_plant() {
    for seed in 1_u64..=20 {
        let delta_a = branch_plant_delta(seed, &[Intervention::SetUvHigh]);
        let delta_b = branch_plant_delta(seed, &[Intervention::SetUvLow]);
        let delta_c = branch_plant_delta(seed, &[Intervention::RemoveFungus, Intervention::SetUvHigh]);

        assert!(
            (delta_a - delta_b).abs() >= 2.5,
            "UV effect too weak for seed {seed}: A={delta_a}, B={delta_b}"
        );
        assert!(
            delta_c.abs() <= delta_a.abs() + 1.2,
            "fungus removal did not weaken UV pathway for seed {seed}: A={delta_a}, C={delta_c}"
        );
    }
}

#[test]
fn signature_nutrient_direct_matches_metadata() {
    for seed in 1_u64..=20 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise(recipe.clone());

        run_ticks(&mut sim, 3);
        let before = sim.state().get(NodeId::PlantPop);
        sim.apply(Intervention::AddNutrient(20.0)).unwrap();
        run_ticks(&mut sim, 3);
        let delta = sim.state().get(NodeId::PlantPop) - before;

        if recipe.metadata.has_nutrient_direct {
            assert!(delta >= 3.0, "expected strong nutrient effect for seed {seed}, delta={delta}");
        } else {
            assert!(delta <= 2.0, "expected weak nutrient effect for seed {seed}, delta={delta}");
        }
    }
}

#[test]
fn signature_toxin_harms_bacteria() {
    for seed in 1_u64..=20 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise(recipe);

        run_ticks(&mut sim, 3);
        let before = sim.state().get(NodeId::BacteriaPop);
        sim.apply(Intervention::AddToxin(20.0)).unwrap();
        run_ticks(&mut sim, 3);
        let delta = sim.state().get(NodeId::BacteriaPop) - before;

        assert!(delta <= -3.0, "toxin did not reduce bacteria for seed {seed}, delta={delta}");
        assert_ne!(delta, 0.0);
    }
}

#[test]
fn diversity_topologies() {
    let mut unique = BTreeSet::new();

    for seed in 1_u64..=50 {
        let recipe = worldgen::generate_playable(seed);
        let mut sig: Vec<(NodeId, NodeId, i8)> = recipe
            .edges
            .iter()
            .map(|edge| {
                let sign = if edge.weight >= 0.0 { 1_i8 } else { -1_i8 };
                (edge.from, edge.to, sign)
            })
            .collect();
        sig.sort_unstable();
        unique.insert(sig);
    }

    assert!(
        unique.len() >= 8,
        "insufficient topology diversity: {} unique signatures",
        unique.len()
    );
}
