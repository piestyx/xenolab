use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::engine::ids::{NodeId, ObjectiveId, NODE_COUNT};
use crate::engine::node::{EdgeSpec, NodeKind};
use crate::engine::world::{
    RecipeMetadata, ThresholdConfig, UvToxinThresholdMode, WorldRecipe, WorldState,
};
use crate::worldgen::spec::{
    archetype_from_seed, incoming_degree_cap, Archetype, EdgeTier, MenuPolarity, MAX_BIAS,
    MIN_BIAS, PRIMARY_MAX_MAG, PRIMARY_MIN_MAG, SECONDARY_MAX_MAG, SECONDARY_MIN_MAG,
    SIGMA_CHEMICAL, SIGMA_ENV, SIGMA_LATENT, SIGMA_ORGANISM, SPICE_MAX_MAG, SPICE_MIN_MAG,
    STABILITY_CAP,
};

#[derive(Debug, Clone, Copy)]
struct PlannedEdge {
    from: NodeId,
    to: NodeId,
    polarity: MenuPolarity,
    tier: EdgeTier,
}

pub fn generate(seed: u64) -> WorldRecipe {
    generate_with_attempt(seed, 0)
}

pub fn generate_with_attempt(seed: u64, attempt: u32) -> WorldRecipe {
    let attempt_seed = hash_seed_attempt(seed, attempt);
    let mut rng = ChaCha8Rng::seed_from_u64(attempt_seed);
    let archetype = archetype_from_seed(seed);
    let threshold = sample_threshold(archetype, &mut rng);

    let mut edges = Vec::new();
    let mut incoming = [0_usize; NODE_COUNT];
    for planned in template_for(archetype) {
        let added = add_edge_tiered(
            &mut edges,
            &mut incoming,
            planned.from,
            planned.to,
            planned.polarity,
            planned.tier,
            &mut rng,
        );
        if !added {
            // Deterministic fallback for this attempt: preserve sparsity/caps.
            return fallback_for_attempt(seed, attempt, archetype);
        }
    }

    if edges.len() < 8 {
        if let Some(optional) = pick_optional_edge(archetype, &mut rng) {
            let _ = add_edge_tiered(
                &mut edges,
                &mut incoming,
                optional.from,
                optional.to,
                optional.polarity,
                optional.tier,
                &mut rng,
            );
        }
    }

    apply_stability_cap(&mut edges);
    edges.sort_by_key(|edge| (edge.from.as_index(), edge.to.as_index()));

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

    debug_assert!(edges.len() >= 6 && edges.len() <= 8);
    debug_assert!(validate_degree_caps(&edges));

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
        threshold,
    }
}

pub fn recipe_hash(recipe: &WorldRecipe) -> blake3::Hash {
    match serde_json::to_vec(recipe) {
        Ok(bytes) => blake3::hash(&bytes),
        Err(_) => blake3::hash(&[]),
    }
}

fn template_for(archetype: Archetype) -> Vec<PlannedEdge> {
    match archetype {
        Archetype::UvSensitive => vec![
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Enzyme,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::BacteriaPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::BacteriaPop,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Secondary,
            },
        ],
        Archetype::NutrientLimited => vec![
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Enzyme,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::BacteriaPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Nutrient,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::PlantPop,
                to: NodeId::Nutrient,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
        ],
        Archetype::ToxinDriven => vec![
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Enzyme,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::BacteriaPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::BacteriaPop,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
        ],
        Archetype::SymbiosisFragile => vec![
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::Enzyme,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::BacteriaPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::BacteriaPop,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::PlantPop,
                to: NodeId::Nutrient,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Secondary,
            },
        ],
        Archetype::DetoxEcosystem => vec![
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Enzyme,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::Enzyme,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Secondary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::BacteriaPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::Toxin,
                to: NodeId::PlantPop,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
            PlannedEdge {
                from: NodeId::BacteriaPop,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Negative,
                tier: EdgeTier::Primary,
            },
        ],
    }
}

fn pick_optional_edge(archetype: Archetype, rng: &mut ChaCha8Rng) -> Option<PlannedEdge> {
    if !rng.gen_bool(0.75) {
        return None;
    }

    let choices: &[PlannedEdge] = match archetype {
        Archetype::UvSensitive => &[PlannedEdge {
            from: NodeId::UvLevel,
            to: NodeId::Toxin,
            polarity: MenuPolarity::Negative,
            tier: EdgeTier::Spice,
        }],
        Archetype::NutrientLimited => &[
            PlannedEdge {
                from: NodeId::BacteriaPop,
                to: NodeId::Nutrient,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Either,
                tier: EdgeTier::Spice,
            },
        ],
        Archetype::ToxinDriven => &[
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Either,
                tier: EdgeTier::Spice,
            },
        ],
        Archetype::SymbiosisFragile => &[PlannedEdge {
            from: NodeId::Nutrient,
            to: NodeId::FungusLoad,
            polarity: MenuPolarity::Positive,
            tier: EdgeTier::Spice,
        }],
        Archetype::DetoxEcosystem => &[
            PlannedEdge {
                from: NodeId::UvLevel,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Either,
                tier: EdgeTier::Spice,
            },
            PlannedEdge {
                from: NodeId::FungusLoad,
                to: NodeId::Toxin,
                polarity: MenuPolarity::Positive,
                tier: EdgeTier::Spice,
            },
        ],
    };

    let idx = rng.gen_range(0..choices.len());
    Some(choices[idx])
}

fn add_edge_tiered(
    edges: &mut Vec<EdgeSpec>,
    incoming: &mut [usize; NODE_COUNT],
    from: NodeId,
    to: NodeId,
    polarity: MenuPolarity,
    tier: EdgeTier,
    rng: &mut ChaCha8Rng,
) -> bool {
    if edges.iter().any(|edge| edge.from == from && edge.to == to) {
        return false;
    }

    let to_idx = to.as_index();
    let cap = incoming_degree_cap(to);
    if incoming[to_idx] >= cap {
        return false;
    }

    let weight = sample_tiered_weight(polarity, tier, rng);
    edges.push(EdgeSpec { from, to, weight });
    incoming[to_idx] += 1;
    true
}

fn sample_tiered_weight(polarity: MenuPolarity, tier: EdgeTier, rng: &mut ChaCha8Rng) -> f32 {
    let (min_mag, max_mag) = match tier {
        EdgeTier::Primary => (PRIMARY_MIN_MAG, PRIMARY_MAX_MAG),
        EdgeTier::Secondary => (SECONDARY_MIN_MAG, SECONDARY_MAX_MAG),
        EdgeTier::Spice => (SPICE_MIN_MAG, SPICE_MAX_MAG),
    };
    let mag = rng.gen_range(min_mag..=max_mag);

    match polarity {
        MenuPolarity::Positive => mag,
        MenuPolarity::Negative => -mag,
        MenuPolarity::Either => {
            if rng.gen_bool(0.5) {
                mag
            } else {
                -mag
            }
        }
    }
}

fn sample_threshold(archetype: Archetype, rng: &mut ChaCha8Rng) -> ThresholdConfig {
    let eligible = matches!(
        archetype,
        Archetype::UvSensitive | Archetype::ToxinDriven | Archetype::DetoxEcosystem
    );
    if !eligible || !rng.gen_bool(0.30) {
        return ThresholdConfig::default();
    }

    let mode = if rng.gen_bool(0.70) {
        UvToxinThresholdMode::Burn
    } else {
        UvToxinThresholdMode::Create
    };

    ThresholdConfig {
        uv_toxin_mode: mode,
        uv_cutoff: 80.0,
        toxin_delta: 2.0,
    }
}

fn fallback_for_attempt(seed: u64, attempt: u32, archetype: Archetype) -> WorldRecipe {
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
            weight: 1.0,
        },
        EdgeSpec {
            from: NodeId::Toxin,
            to: NodeId::BacteriaPop,
            weight: -0.4,
        },
        EdgeSpec {
            from: NodeId::Toxin,
            to: NodeId::PlantPop,
            weight: -1.0,
        },
        EdgeSpec {
            from: NodeId::BacteriaPop,
            to: NodeId::Toxin,
            weight: -1.0,
        },
    ];
    apply_stability_cap(&mut edges);
    edges.sort_by_key(|edge| (edge.from.as_index(), edge.to.as_index()));

    let metadata = derive_metadata(&edges);
    let mut noise_sigma = [0.0; NODE_COUNT];
    for node in NodeId::ALL {
        noise_sigma[node.as_index()] = match node_kind(node) {
            NodeKind::Env => SIGMA_ENV,
            NodeKind::Organism => SIGMA_ORGANISM,
            NodeKind::Chemical => SIGMA_CHEMICAL,
            NodeKind::Latent => SIGMA_LATENT,
        };
    }

    WorldRecipe {
        seed,
        attempt,
        archetype,
        objective: ObjectiveId::for_seed(seed),
        node_specs: crate::engine::node::node_catalog(),
        edges,
        biases: [0.0; NODE_COUNT],
        noise_sigma,
        initial_state: WorldState {
            values: [50.0, 45.0, 45.0, 50.0, 25.0, 50.0, 30.0],
        },
        metadata,
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

fn validate_degree_caps(edges: &[EdgeSpec]) -> bool {
    let mut incoming = [0_usize; NODE_COUNT];
    for edge in edges {
        let idx = edge.to.as_index();
        incoming[idx] += 1;
        if incoming[idx] > incoming_degree_cap(edge.to) {
            return false;
        }
    }
    incoming[NodeId::Enzyme.as_index()] == 2
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
