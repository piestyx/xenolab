use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::engine::ids::{NodeId, ObjectiveId, NODE_COUNT};
use crate::engine::node::{node_catalog, EdgeSpec, NodeSpec};
use crate::worldgen::spec::Archetype;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UvToxinThresholdMode {
    None,
    Burn,
    Create,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ThresholdConfig {
    pub uv_toxin_mode: UvToxinThresholdMode,
    pub uv_cutoff: f32,
    pub toxin_delta: f32,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            uv_toxin_mode: UvToxinThresholdMode::None,
            uv_cutoff: 80.0,
            toxin_delta: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct RecipeMetadata {
    pub has_nutrient_direct: bool,
    pub has_uv_toxin: bool,
    pub has_bacteria_toxin_decay: bool,
    pub has_fungus_toxin_prod: bool,
    pub has_plant_nutrient_deplete: bool,
    pub has_bacteria_nutrient_recycle: bool,
    pub has_twist_toxin_fungus: bool,
    pub has_twist_nutrient_fungus: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorldRecipe {
    pub seed: u64,
    pub attempt: u32,
    pub archetype: Archetype,
    pub objective: ObjectiveId,
    pub node_specs: [NodeSpec; NODE_COUNT],
    pub edges: Vec<EdgeSpec>,
    pub biases: [f32; NODE_COUNT],
    pub noise_sigma: [f32; NODE_COUNT],
    pub initial_state: WorldState,
    pub metadata: RecipeMetadata,
    pub threshold: ThresholdConfig,
}

impl WorldRecipe {
    pub fn placeholder(seed: u64, attempt: u32) -> Self {
        let mut noise_sigma = [0.5; NODE_COUNT];
        noise_sigma[NodeId::UvLevel.as_index()] = 0.0;
        noise_sigma[NodeId::Toxin.as_index()] = 0.8;
        noise_sigma[NodeId::Nutrient.as_index()] = 0.8;
        noise_sigma[NodeId::Enzyme.as_index()] = 0.6;

        Self {
            seed,
            attempt,
            archetype: crate::worldgen::spec::archetype_from_seed(seed),
            objective: ObjectiveId::for_seed(seed),
            node_specs: node_catalog(),
            edges: Vec::new(),
            biases: [0.0; NODE_COUNT],
            noise_sigma,
            initial_state: WorldState::default(),
            metadata: RecipeMetadata::default(),
            threshold: ThresholdConfig::default(),
        }
    }

    pub fn recipe_hash(&self) -> Result<blake3::Hash, WorldError> {
        let bytes = serde_json::to_vec(self)?;
        Ok(blake3::hash(&bytes))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WorldState {
    pub values: [f32; NODE_COUNT],
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            values: [50.0, 50.0, 50.0, 50.0, 20.0, 50.0, 50.0],
        }
    }
}

impl WorldState {
    pub fn get(&self, node: NodeId) -> f32 {
        self.values[node.as_index()]
    }

    pub fn set(&mut self, node: NodeId, value: f32) {
        self.values[node.as_index()] = clamp_0_100(value);
    }

    pub fn with_uv(mut self, value: f32) -> Self {
        self.set(NodeId::UvLevel, value);
        self
    }
}

pub fn clamp_0_100(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("failed to serialize world recipe: {0}")]
    Serialize(#[from] serde_json::Error),
}
