use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::engine::ids::{NodeId, ObjectiveId, NODE_COUNT};
use crate::engine::node::{EdgeSpec, NodeKind};
use crate::engine::world::{RecipeMetadata, ThresholdConfig, WorldRecipe, WorldState};
use crate::worldgen::spec::{
    CONFOUNDER_MENU, FEEDBACK_MENU, MANDATORY_EDGES, MAX_BIAS, MAX_NEG_WEIGHT, MAX_POS_WEIGHT,
    MAX_TWIST_MAGNITUDE, MenuPolarity, MIN_BIAS, MIN_NEG_WEIGHT, MIN_POS_WEIGHT,
    MIN_TWIST_MAGNITUDE, SIGMA_CHEMICAL, SIGMA_ENV, SIGMA_LATENT, SIGMA_ORGANISM, STABILITY_CAP,
    TWIST_MENU, pick_archetype,
};

pub fn generate(seed: u64) -> WorldRecipe {
    generate_with_attempt(seed, 0)
}

pub fn generate_with_attempt(seed: u64, attempt: u32) -> WorldRecipe {
    let attempt_seed = hash_seed_attempt(seed, attempt);
    let mut rng = ChaCha8Rng::seed_from_u64(attempt_seed);
    let archetype = pick_archetype(&mut rng);

    let k_feedback = rng.gen_range(1..=2);
    let k_confound = rng.gen_range(1..=2);
    let k_twist = rng.gen_range(0..=1);

    let mut feedback_idx = choose_indices(FEEDBACK_MENU.len(), k_feedback, &mut rng);
    let mut confound_idx = choose_indices(CONFOUNDER_MENU.len(), k_confound, &mut rng);
    let twist_idx = choose_indices(TWIST_MENU.len(), k_twist, &mut rng);

    enforce_nutrient_constraint(&mut feedback_idx, &mut confound_idx, &mut rng);

    feedback_idx.sort_unstable();
    confound_idx.sort_unstable();

    let mut edges = Vec::new();
    for item in MANDATORY_EDGES {
        edges.push(EdgeSpec {
            from: item.from,
            to: item.to,
            weight: sample_weight(item.polarity, item.is_twist, &mut rng),
        });
    }
    for idx in feedback_idx {
        let item = FEEDBACK_MENU[idx];
        edges.push(EdgeSpec {
            from: item.from,
            to: item.to,
            weight: sample_weight(item.polarity, item.is_twist, &mut rng),
        });
    }
    for idx in confound_idx {
        let item = CONFOUNDER_MENU[idx];
        edges.push(EdgeSpec {
            from: item.from,
            to: item.to,
            weight: sample_weight(item.polarity, item.is_twist, &mut rng),
        });
    }
    for idx in twist_idx {
        let item = TWIST_MENU[idx];
        edges.push(EdgeSpec {
            from: item.from,
            to: item.to,
            weight: sample_weight(item.polarity, item.is_twist, &mut rng),
        });
    }

    apply_stability_cap(&mut edges);
    edges.sort_by_key(|edge| (edge.from.as_index(), edge.to.as_index()));
    debug_assert!(has_edge(&edges, NodeId::Toxin, NodeId::PlantPop));

    let metadata = derive_metadata(&edges);
    let biases = sample_biases(&mut rng);

    let mut noise_sigma = [0.0; NODE_COUNT];
    for node in NodeId::ALL {
        noise_sigma[node.as_index()] = match node_kind(node) {
            NodeKind::Env => SIGMA_ENV,
            NodeKind::Organism => SIGMA_ORGANISM,
            NodeKind::Chemical => SIGMA_CHEMICAL,
            NodeKind::Latent => SIGMA_LATENT,
        };
    }

    let initial_state = WorldState {
        values: [
            50.0,
            rng.gen_range(40.0..=60.0),
            rng.gen_range(35.0..=65.0),
            rng.gen_range(35.0..=65.0),
            rng.gen_range(15.0..=35.0),
            rng.gen_range(40.0..=75.0),
            rng.gen_range(40.0..=60.0),
        ],
    };

    WorldRecipe {
        seed,
        attempt,
        archetype,
        objective: ObjectiveId::for_seed(seed),
        node_specs: crate::engine::node::node_catalog(),
        edges,
        biases,
        noise_sigma,
        initial_state,
        metadata,
        threshold: ThresholdConfig::default(),
    }
}

pub fn recipe_hash(recipe: &WorldRecipe) -> blake3::Hash {
    match serde_json::to_vec(recipe) {
        Ok(bytes) => blake3::hash(&bytes),
        Err(_) => blake3::hash(&[]),
    }
}

fn choose_indices(len: usize, k: usize, rng: &mut ChaCha8Rng) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..len).collect();
    idx.shuffle(rng);
    idx.truncate(k.min(len));
    idx
}

fn enforce_nutrient_constraint(
    feedback_idx: &mut Vec<usize>,
    confound_idx: &mut Vec<usize>,
    rng: &mut ChaCha8Rng,
) {
    let has_nutrient_direct = confound_idx.contains(&0);
    if has_nutrient_direct {
        return;
    }

    let has_plant_nutrient = feedback_idx.contains(&1);
    let has_bacteria_nutrient = confound_idx.contains(&2);
    if has_plant_nutrient || has_bacteria_nutrient {
        return;
    }

    let choose_feedback = rng.gen_bool(0.5);
    if choose_feedback {
        if feedback_idx.is_empty() {
            feedback_idx.push(1);
        } else {
            feedback_idx[0] = 1;
        }
    } else if confound_idx.is_empty() {
        confound_idx.push(2);
    } else {
        confound_idx[0] = 2;
    }
}

fn sample_weight(polarity: MenuPolarity, is_twist: bool, rng: &mut ChaCha8Rng) -> f32 {
    if is_twist {
        let magnitude = rng.gen_range(MIN_TWIST_MAGNITUDE..=MAX_TWIST_MAGNITUDE);
        return match polarity {
            MenuPolarity::Positive => magnitude,
            MenuPolarity::Negative => -magnitude,
            MenuPolarity::Either => {
                if rng.gen_bool(0.5) {
                    magnitude
                } else {
                    -magnitude
                }
            }
        };
    }

    match polarity {
        MenuPolarity::Positive => rng.gen_range(MIN_POS_WEIGHT..=MAX_POS_WEIGHT),
        MenuPolarity::Negative => rng.gen_range(MIN_NEG_WEIGHT..=MAX_NEG_WEIGHT),
        MenuPolarity::Either => {
            if rng.gen_bool(0.5) {
                rng.gen_range(MIN_POS_WEIGHT..=MAX_POS_WEIGHT)
            } else {
                rng.gen_range(MIN_NEG_WEIGHT..=MAX_NEG_WEIGHT)
            }
        }
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

fn derive_metadata(edges: &[EdgeSpec]) -> RecipeMetadata {
    let mut metadata = RecipeMetadata::default();
    for edge in edges {
        let from = edge.from;
        let to = edge.to;
        if from == NodeId::Nutrient && to == NodeId::PlantPop {
            metadata.has_nutrient_direct = true;
        }
        if from == NodeId::UvLevel && to == NodeId::Toxin {
            metadata.has_uv_toxin = true;
        }
        if from == NodeId::BacteriaPop && to == NodeId::Toxin {
            metadata.has_bacteria_toxin_decay = true;
        }
        if from == NodeId::FungusLoad && to == NodeId::Toxin {
            metadata.has_fungus_toxin_prod = true;
        }
        if from == NodeId::PlantPop && to == NodeId::Nutrient {
            metadata.has_plant_nutrient_deplete = true;
        }
        if from == NodeId::BacteriaPop && to == NodeId::Nutrient {
            metadata.has_bacteria_nutrient_recycle = true;
        }
        if from == NodeId::Toxin && to == NodeId::FungusLoad {
            metadata.has_twist_toxin_fungus = true;
        }
        if from == NodeId::Nutrient && to == NodeId::FungusLoad {
            metadata.has_twist_nutrient_fungus = true;
        }
    }
    metadata
}

fn sample_biases(rng: &mut ChaCha8Rng) -> [f32; NODE_COUNT] {
    let mut biases = [0.0; NODE_COUNT];
    for node in NodeId::ALL {
        biases[node.as_index()] = rng.gen_range(MIN_BIAS..=MAX_BIAS);
    }
    biases
}

fn hash_seed_attempt(seed: u64, attempt: u32) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(&attempt.to_le_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

fn node_kind(node: NodeId) -> NodeKind {
    match node {
        NodeId::UvLevel => NodeKind::Env,
        NodeId::PlantPop | NodeId::FungusLoad | NodeId::BacteriaPop => NodeKind::Organism,
        NodeId::Toxin | NodeId::Nutrient => NodeKind::Chemical,
        NodeId::Enzyme => NodeKind::Latent,
    }
}

fn has_edge(edges: &[EdgeSpec], from: NodeId, to: NodeId) -> bool {
    edges.iter().any(|edge| edge.from == from && edge.to == to)
}
