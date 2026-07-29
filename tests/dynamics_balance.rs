use xenolab::engine::ids::NodeId;
use xenolab::engine::interventions::Intervention;
use xenolab::engine::sim::Simulator;
use xenolab::worldgen;

fn run_ticks(sim: &mut Simulator, count: usize) {
    for _ in 0..count {
        sim.apply(Intervention::AdvanceTime).unwrap();
    }
}

#[test]
fn baseline_has_pressure() {
    let mut failed = Vec::new();

    for seed in 1_u64..=30 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe);
        let mut saw_pressure = false;

        for _ in 0..60 {
            sim.apply(Intervention::AdvanceTime).unwrap();
            let plant = sim.state().get(NodeId::PlantPop);
            let toxin = sim.state().get(NodeId::Toxin);
            let bacteria = sim.state().get(NodeId::BacteriaPop);
            if plant < 80.0 || toxin > 20.0 || bacteria < 20.0 {
                saw_pressure = true;
                break;
            }
        }

        if !saw_pressure {
            failed.push(seed);
        }
    }

    assert!(
        failed.is_empty(),
        "baseline pressure check failed; no pressure signals for seeds {:?}",
        failed
    );
}

#[test]
fn plant_not_saturated_forever() {
    let mut failed = Vec::new();

    for seed in 1_u64..=30 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe);

        let mut current_streak = 0_u32;
        let mut longest_streak = 0_u32;

        for _ in 0..80 {
            sim.apply(Intervention::AdvanceTime).unwrap();
            let plant = sim.state().get(NodeId::PlantPop);
            if plant >= 95.0 {
                current_streak += 1;
                longest_streak = longest_streak.max(current_streak);
            } else {
                current_streak = 0;
            }
        }

        if longest_streak > 30 {
            failed.push((seed, longest_streak));
        }
    }

    assert!(
        failed.is_empty(),
        "plant saturation streak too long for seeds {:?}",
        failed
    );
}

#[test]
fn toxin_can_hurt_plant() {
    let mut pass_count = 0_u32;
    let mut failed = Vec::new();

    for seed in 10_u64..=20 {
        let recipe = worldgen::generate_playable(seed);
        let mut sim = Simulator::new_no_noise_for_analysis(recipe);

        run_ticks(&mut sim, 3);
        let plant_before = sim.state().get(NodeId::PlantPop);

        sim.apply(Intervention::AddToxin(20.0)).unwrap();
        let mut min_plant = sim.state().get(NodeId::PlantPop);
        for _ in 0..6 {
            sim.apply(Intervention::AdvanceTime).unwrap();
            min_plant = min_plant.min(sim.state().get(NodeId::PlantPop));
        }
        let plant_after = sim.state().get(NodeId::PlantPop);

        let harmed = min_plant <= plant_before - 1.0 || plant_after <= plant_before + 2.0;
        if harmed {
            pass_count += 1;
        } else {
            failed.push(format!(
                "seed {seed}: before={plant_before:.3}, min={min_plant:.3}, after={plant_after:.3}"
            ));
        }
    }

    assert!(
        pass_count >= 8,
        "toxin->plant coupling weak: pass_count={}, failures={:?}",
        pass_count,
        failed
    );
}
