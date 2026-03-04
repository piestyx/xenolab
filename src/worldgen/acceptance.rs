use crate::engine::ids::NodeId;
use crate::engine::interventions::Intervention;
use crate::engine::sim::Simulator;
use crate::engine::world::WorldRecipe;
use crate::worldgen::generator;
use crate::worldgen::spec::ACCEPTANCE_ATTEMPTS;

pub fn generate_playable(seed: u64) -> WorldRecipe {
    let mut fallback_no_saturation = None;

    for attempt in 0..ACCEPTANCE_ATTEMPTS {
        let recipe = generator::generate_with_attempt(seed, attempt);
        if is_playable(&recipe) {
            return recipe;
        }
        if fallback_no_saturation.is_none() && check_no_noise_saturation(&recipe) {
            fallback_no_saturation = Some(recipe);
        }
    }

    fallback_no_saturation.unwrap_or_else(|| generator::generate_with_attempt(seed, 0))
}

fn is_playable(recipe: &WorldRecipe) -> bool {
    check_structure(recipe) && check_stability(recipe) && check_toxin_signature(recipe)
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
