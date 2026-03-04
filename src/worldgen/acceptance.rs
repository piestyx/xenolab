use crate::engine::ids::{NodeId, ObjectiveId, NODE_COUNT};
use crate::engine::interventions::Intervention;
use crate::engine::node::{node_catalog, EdgeSpec};
use crate::engine::sim::Simulator;
use crate::engine::world::{RecipeMetadata, ThresholdConfig, WorldRecipe, WorldState};
use crate::worldgen::generator;
use crate::worldgen::spec::{
    ACCEPTANCE_ATTEMPTS, SIGMA_CHEMICAL, SIGMA_ENV, SIGMA_LATENT, SIGMA_ORGANISM, STABILITY_CAP,
    archetype_from_seed,
};

pub fn generate_playable(seed: u64) -> WorldRecipe {
    for attempt in 0..ACCEPTANCE_ATTEMPTS {
        let recipe = generator::generate_with_attempt(seed, attempt);
        if is_playable(&recipe) {
            return recipe;
        }
    }
    generator::generate_with_attempt(seed, 0)
}

fn is_playable(recipe: &WorldRecipe) -> bool {
    check_structure(recipe)
        && check_stability(recipe)
        && check_uv_signature(recipe)
        && check_nutrient_signature(recipe)
        && check_toxin_signature(recipe)
}

fn check_structure(recipe: &WorldRecipe) -> bool {
    let has_plant_growth_path = recipe
        .edges
        .iter()
        .any(|edge| edge.from == NodeId::Enzyme && edge.to == NodeId::PlantPop && edge.weight > 0.0);
    let has_plant_hazard_path = recipe
        .edges
        .iter()
        .any(|edge| edge.from == NodeId::Toxin && edge.to == NodeId::PlantPop && edge.weight < 0.0);
    let has_toxin_loop = recipe
        .edges
        .iter()
        .any(|edge| edge.from == NodeId::BacteriaPop && edge.to == NodeId::Toxin);

    has_plant_growth_path && has_plant_hazard_path && has_toxin_loop
}

fn check_stability(recipe: &WorldRecipe) -> bool {
    let mut sim = Simulator::new(recipe.clone());
    let start = *sim.state();
    if run_ticks(&mut sim, 30).is_err() {
        return false;
    }
    for value in sim.state().values {
        if !(0.0..=100.0).contains(&value) {
            return false;
        }
    }
    let plant_delta = (sim.state().get(NodeId::PlantPop) - start.get(NodeId::PlantPop)).abs();
    let toxin_delta = (sim.state().get(NodeId::Toxin) - start.get(NodeId::Toxin)).abs();
    let bacteria_delta = (sim.state().get(NodeId::BacteriaPop) - start.get(NodeId::BacteriaPop)).abs();
    (plant_delta >= 5.0 || toxin_delta >= 5.0 || bacteria_delta >= 5.0)
        && check_no_noise_saturation(recipe)
}

fn check_uv_signature(recipe: &WorldRecipe) -> bool {
    let delta_a = branch_plant_delta(recipe, &[Intervention::SetUvHigh]);
    let delta_b = branch_plant_delta(recipe, &[Intervention::SetUvLow]);
    let delta_c = branch_plant_delta(recipe, &[Intervention::RemoveFungus, Intervention::SetUvHigh]);
    match (delta_a, delta_b, delta_c) {
        (Some(a), Some(b), Some(c)) => (a - b).abs() >= 3.0 && c.abs() <= a.abs() + 1.0,
        _ => false,
    }
}

fn check_nutrient_signature(recipe: &WorldRecipe) -> bool {
    let mut sim = Simulator::new(recipe.clone());
    if run_ticks(&mut sim, 3).is_err() {
        return false;
    }
    let plant_before = sim.state().get(NodeId::PlantPop);
    if sim.apply(Intervention::AddNutrient(20.0)).is_err() {
        return false;
    }
    if run_ticks(&mut sim, 3).is_err() {
        return false;
    }
    let delta = sim.state().get(NodeId::PlantPop) - plant_before;
    if recipe.metadata.has_nutrient_direct {
        delta >= 3.0
    } else {
        delta <= 2.0
    }
}

fn check_toxin_signature(recipe: &WorldRecipe) -> bool {
    let mut sim = Simulator::new(recipe.clone());
    if run_ticks(&mut sim, 3).is_err() {
        return false;
    }
    let before = sim.state().get(NodeId::BacteriaPop);
    if sim.apply(Intervention::AddToxin(20.0)).is_err() {
        return false;
    }
    if run_ticks(&mut sim, 3).is_err() {
        return false;
    }
    let delta = sim.state().get(NodeId::BacteriaPop) - before;
    delta <= -3.0
}

fn branch_plant_delta(recipe: &WorldRecipe, interventions: &[Intervention]) -> Option<f32> {
    let mut sim = Simulator::new(recipe.clone());
    run_ticks(&mut sim, 3).ok()?;
    let before = sim.state().get(NodeId::PlantPop);
    for action in interventions {
        sim.apply(action.clone()).ok()?;
    }
    run_ticks(&mut sim, 3).ok()?;
    Some(sim.state().get(NodeId::PlantPop) - before)
}

fn run_ticks(sim: &mut Simulator, count: usize) -> Result<(), ()> {
    for _ in 0..count {
        sim.apply(Intervention::AdvanceTime).map_err(|_| ())?;
    }
    Ok(())
}

fn check_no_noise_saturation(recipe: &WorldRecipe) -> bool {
    let mut sim = Simulator::new_no_noise(recipe.clone());
    let mut current_streak = 0_u32;
    let mut max_streak = 0_u32;

    for _ in 0..80 {
        if sim.apply(Intervention::AdvanceTime).is_err() {
            return false;
        }
        let plant = sim.state().get(NodeId::PlantPop);
        if plant >= 95.0 {
            current_streak += 1;
            max_streak = max_streak.max(current_streak);
        } else {
            current_streak = 0;
        }
    }

    max_streak <= 30
}

fn fallback_recipe(seed: u64) -> WorldRecipe {
    let mut edges = vec![
        EdgeSpec {
            from: NodeId::FungusLoad,
            to: NodeId::Enzyme,
            weight: 0.6,
        },
        EdgeSpec {
            from: NodeId::UvLevel,
            to: NodeId::Enzyme,
            weight: 1.0,
        },
        EdgeSpec {
            from: NodeId::Enzyme,
            to: NodeId::PlantPop,
            weight: 0.6,
        },
        EdgeSpec {
            from: NodeId::Toxin,
            to: NodeId::BacteriaPop,
            weight: -1.1,
        },
        EdgeSpec {
            from: NodeId::Toxin,
            to: NodeId::PlantPop,
            weight: -1.0,
        },
        EdgeSpec {
            from: NodeId::BacteriaPop,
            to: NodeId::Toxin,
            weight: -0.5,
        },
        EdgeSpec {
            from: NodeId::FungusLoad,
            to: NodeId::Toxin,
            weight: 0.7,
        },
        EdgeSpec {
            from: NodeId::PlantPop,
            to: NodeId::Nutrient,
            weight: -0.7,
        },
        EdgeSpec {
            from: NodeId::Nutrient,
            to: NodeId::PlantPop,
            weight: 0.5,
        },
    ];
    apply_stability_cap(&mut edges);
    edges.sort_by_key(|edge| (edge.from.as_index(), edge.to.as_index()));

    let mut noise_sigma = [0.0; NODE_COUNT];
    noise_sigma[NodeId::UvLevel.as_index()] = SIGMA_ENV;
    noise_sigma[NodeId::PlantPop.as_index()] = SIGMA_ORGANISM;
    noise_sigma[NodeId::FungusLoad.as_index()] = SIGMA_ORGANISM;
    noise_sigma[NodeId::BacteriaPop.as_index()] = SIGMA_ORGANISM;
    noise_sigma[NodeId::Toxin.as_index()] = SIGMA_CHEMICAL;
    noise_sigma[NodeId::Nutrient.as_index()] = SIGMA_CHEMICAL;
    noise_sigma[NodeId::Enzyme.as_index()] = SIGMA_LATENT;

    WorldRecipe {
        seed,
        attempt: u32::MAX,
        archetype: archetype_from_seed(seed),
        objective: ObjectiveId::for_seed(seed),
        node_specs: node_catalog(),
        edges,
        biases: [0.0; NODE_COUNT],
        noise_sigma,
        initial_state: WorldState {
            values: [50.0, 45.0, 45.0, 50.0, 25.0, 50.0, 30.0],
        },
        metadata: RecipeMetadata {
            has_nutrient_direct: true,
            has_uv_toxin: false,
            has_bacteria_toxin_decay: true,
            has_fungus_toxin_prod: true,
            has_plant_nutrient_deplete: true,
            has_bacteria_nutrient_recycle: false,
            has_twist_toxin_fungus: false,
            has_twist_nutrient_fungus: false,
        },
        threshold: ThresholdConfig::default(),
    }
}

fn apply_stability_cap(edges: &mut [EdgeSpec]) {
    for target in NodeId::ALL {
        let total_abs: f32 = edges
            .iter()
            .filter(|edge| edge.to == target)
            .map(|edge| edge.weight.abs())
            .sum();
        if total_abs <= STABILITY_CAP || total_abs <= f32::EPSILON {
            continue;
        }
        let scale = STABILITY_CAP / total_abs;
        for edge in edges.iter_mut().filter(|edge| edge.to == target) {
            edge.weight *= scale;
        }
    }
}
