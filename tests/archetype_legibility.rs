use std::collections::{BTreeMap, BTreeSet};

use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::sim::Simulator;
use xenolab::engine::world::UvToxinThresholdMode;
use xenolab::worldgen;
use xenolab::worldgen::spec::{incoming_degree_cap, Archetype, INFLUENCE_SCALE};

fn run_ticks(sim: &mut Simulator, count: usize) {
    for _ in 0..count {
        sim.apply(Intervention::AdvanceTime).unwrap();
    }
}

fn plant_delta(seed: u64, actions: &[Intervention], post_ticks: usize) -> f32 {
    let recipe = worldgen::generate_playable(seed);
    let mut sim = Simulator::new_no_noise_for_analysis(recipe);
    run_ticks(&mut sim, 3);
    let before = sim.state().get(NodeId::PlantPop);
    for action in actions {
        sim.apply(action.clone()).unwrap();
    }
    run_ticks(&mut sim, post_ticks);
    sim.state().get(NodeId::PlantPop) - before
}

#[test]
fn sparsity_and_degree_caps() {
    let mut failures = Vec::new();

    for seed in 1_u64..=50 {
        let recipe = worldgen::generate_playable(seed);
        if !(6..=8).contains(&recipe.edges.len()) {
            failures.push(format!("seed {seed}: edge_count={}", recipe.edges.len()));
            continue;
        }

        let mut incoming = [0_usize; xenolab::engine::ids::NODE_COUNT];
        for edge in &recipe.edges {
            incoming[edge.to.as_index()] += 1;
        }

        let enzyme = incoming[NodeId::Enzyme.as_index()];
        let plant = incoming[NodeId::PlantPop.as_index()];
        let toxin = incoming[NodeId::Toxin.as_index()];
        let nutrient = incoming[NodeId::Nutrient.as_index()];
        let bacteria = incoming[NodeId::BacteriaPop.as_index()];
        let fungus = incoming[NodeId::FungusLoad.as_index()];

        let caps_ok = enzyme == 2
            && plant <= incoming_degree_cap(NodeId::PlantPop)
            && toxin <= incoming_degree_cap(NodeId::Toxin)
            && nutrient <= incoming_degree_cap(NodeId::Nutrient)
            && bacteria <= incoming_degree_cap(NodeId::BacteriaPop)
            && fungus <= incoming_degree_cap(NodeId::FungusLoad);

        if !caps_ok {
            failures.push(format!(
                "seed {seed}: deg enzyme={enzyme} plant={plant} toxin={toxin} nutrient={nutrient} bacteria={bacteria} fungus={fungus}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "sparsity/degree-cap failures: {:?}",
        failures
    );
}

#[test]
fn archetype_rotation_exists() {
    let mut unique = BTreeSet::new();
    for seed in 1_u64..=80 {
        unique.insert(worldgen::generate_playable(seed).archetype);
    }

    assert!(
        unique.len() >= 4,
        "insufficient archetype rotation: seen {:?}",
        unique
    );
}

#[test]
fn dominance_ratio_for_plant() {
    let mut failures = Vec::new();

    for seed in 1_u64..=50 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe.clone());

        let mut contributions = [0.0_f32; xenolab::engine::ids::NODE_COUNT];
        for _ in 0..5 {
            let state = *sim.state();
            for edge in recipe
                .edges
                .iter()
                .filter(|edge| edge.to == NodeId::PlantPop)
            {
                let from_idx = edge.from.as_index();
                let parent_norm = state.values[from_idx] / 100.0;
                let c = (edge.weight * parent_norm * INFLUENCE_SCALE).abs();
                contributions[from_idx] += c;
            }
            sim.apply(Intervention::AdvanceTime).unwrap();
        }

        let mut ordered: Vec<f32> = contributions.iter().copied().filter(|c| *c > 0.0).collect();
        if ordered.is_empty() {
            failures.push(format!("seed {seed}: zero contributions"));
            continue;
        }
        ordered.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let total: f32 = ordered.iter().sum();
        let top1 = ordered[0];
        let top2 = *ordered.get(1).unwrap_or(&0.0);
        let dominance = (top1 + top2) / total.max(f32::EPSILON);

        if dominance < 0.60 {
            failures.push(format!(
                "seed {seed}: dominance={dominance:.3} total={total:.3} ordered={:?}",
                ordered
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "plant dominance ratio failures: {:?}",
        failures
    );
}

#[test]
fn archetype_signature_dominant_constraint() {
    let mut picked: BTreeMap<Archetype, Vec<u64>> = BTreeMap::new();

    for seed in 1_u64..=200 {
        let archetype = worldgen::generate_playable(seed).archetype;
        let entry = picked.entry(archetype).or_default();
        if entry.len() < 3 {
            entry.push(seed);
        }
    }

    for archetype in [
        Archetype::UvSensitive,
        Archetype::NutrientLimited,
        Archetype::ToxinDriven,
        Archetype::SymbiosisFragile,
        Archetype::DetoxEcosystem,
    ] {
        assert!(
            picked.get(&archetype).map(|v| v.len()).unwrap_or(0) >= 3,
            "missing representative seeds for {:?}: {:?}",
            archetype,
            picked.get(&archetype)
        );
    }

    for seed in picked.get(&Archetype::UvSensitive).unwrap() {
        let duv = plant_delta(*seed, &[Intervention::SetUvHigh], 3);
        let dnut = plant_delta(*seed, &[Intervention::AddNutrient(20.0)], 3);
        assert!(
            duv >= dnut + 0.5,
            "UvSensitive seed {seed}: duv={duv:.3}, dnut={dnut:.3}"
        );
    }

    for seed in picked.get(&Archetype::NutrientLimited).unwrap() {
        let duv = plant_delta(*seed, &[Intervention::SetUvHigh], 3);
        let dnut = plant_delta(*seed, &[Intervention::AddNutrient(20.0)], 3);
        assert!(
            dnut >= duv + 1.0,
            "NutrientLimited seed {seed}: duv={duv:.3}, dnut={dnut:.3}"
        );
    }

    for seed in picked.get(&Archetype::SymbiosisFragile).unwrap() {
        let with_fungus_uv = plant_delta(*seed, &[Intervention::SetUvHigh], 3);
        let no_fungus_uv = plant_delta(
            *seed,
            &[Intervention::RemoveFungus, Intervention::SetUvHigh],
            3,
        );
        let no_fungus_control = plant_delta(*seed, &[Intervention::RemoveFungus], 3);
        let uv_lift_no_fungus = no_fungus_uv - no_fungus_control;
        assert!(
            with_fungus_uv >= uv_lift_no_fungus + 1.0,
            "SymbiosisFragile seed {seed}: with_uv={with_fungus_uv:.3}, no_fungus_uv={no_fungus_uv:.3}, no_fungus_control={no_fungus_control:.3}, uv_lift_no_fungus={uv_lift_no_fungus:.3}"
        );
    }

    for archetype in [Archetype::ToxinDriven, Archetype::DetoxEcosystem] {
        let seeds = picked.get(&archetype).unwrap();
        let mut pass_count = 0_u32;
        let mut failures = Vec::new();

        for seed in seeds {
            let recipe = worldgen::generate_playable(*seed);
            let mut sim = Simulator::new_no_noise_for_analysis(recipe);
            run_ticks(&mut sim, 3);
            let plant_before = sim.state().get(NodeId::PlantPop);
            let bacteria_before = sim.state().get(NodeId::BacteriaPop);

            sim.apply(Intervention::AddToxin(20.0)).unwrap();
            run_ticks(&mut sim, 6);

            let plant_after = sim.state().get(NodeId::PlantPop);
            let bacteria_after = sim.state().get(NodeId::BacteriaPop);
            let ok = plant_after <= plant_before + 2.0 || bacteria_after <= bacteria_before - 1.0;
            if ok {
                pass_count += 1;
            } else {
                failures.push(format!(
                    "seed {seed}: plant_before={plant_before:.3} plant_after={plant_after:.3} bacteria_before={bacteria_before:.3} bacteria_after={bacteria_after:.3}"
                ));
            }
        }

        assert!(
            pass_count >= 2,
            "{:?} constraint signature weak: pass_count={}, failures={:?}",
            archetype,
            pass_count,
            failures
        );
    }
}

#[test]
fn threshold_behavior_if_enabled() {
    let mut seen_threshold = 0_u32;
    let mut failures = Vec::new();

    for seed in 1_u64..=100 {
        let recipe = worldgen::generate_playable(seed);
        let threshold = recipe.threshold;
        if threshold.uv_toxin_mode == UvToxinThresholdMode::None {
            continue;
        }
        seen_threshold += 1;

        let mut baseline = Simulator::new_no_noise_for_analysis(recipe.clone());
        baseline.apply(Intervention::AdvanceTime).unwrap();
        run_ticks(&mut baseline, 3);
        let toxin_baseline = baseline.state().get(NodeId::Toxin);

        let mut high_uv = Simulator::new_no_noise_for_analysis(recipe);
        high_uv.apply(Intervention::SetUvHigh).unwrap();
        run_ticks(&mut high_uv, 3);
        let toxin_high = high_uv.state().get(NodeId::Toxin);

        match threshold.uv_toxin_mode {
            UvToxinThresholdMode::Burn => {
                if toxin_high > toxin_baseline - 0.5 {
                    failures.push(format!(
                        "seed {seed}: Burn expected lower toxin, baseline={toxin_baseline:.3}, high_uv={toxin_high:.3}"
                    ));
                }
            }
            UvToxinThresholdMode::Create => {
                if toxin_high < toxin_baseline + 0.5 {
                    failures.push(format!(
                        "seed {seed}: Create expected higher toxin, baseline={toxin_baseline:.3}, high_uv={toxin_high:.3}"
                    ));
                }
            }
            UvToxinThresholdMode::None => {}
        }
    }

    assert!(
        seen_threshold > 0,
        "no threshold-enabled worlds found in seeds 1..100"
    );
    assert!(
        failures.is_empty(),
        "threshold behavior failures: {:?}",
        failures
    );
}
