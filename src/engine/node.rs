use serde::Serialize;

use crate::engine::ids::{NodeId, NODE_COUNT};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum NodeKind {
    Env,
    Organism,
    Chemical,
    Latent,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct NodeSpec {
    pub id: NodeId,
    pub stable_name: &'static str,
    pub kind: NodeKind,
    pub observable: bool,
}

impl NodeSpec {
    pub const fn new(
        id: NodeId,
        stable_name: &'static str,
        kind: NodeKind,
        observable: bool,
    ) -> Self {
        Self {
            id,
            stable_name,
            kind,
            observable,
        }
    }
}

pub const fn node_catalog() -> [NodeSpec; NODE_COUNT] {
    [
        NodeSpec::new(NodeId::UvLevel, "uv_level", NodeKind::Env, true),
        NodeSpec::new(NodeId::PlantPop, "plant_pop", NodeKind::Organism, true),
        NodeSpec::new(NodeId::FungusLoad, "fungus_load", NodeKind::Organism, true),
        NodeSpec::new(
            NodeId::BacteriaPop,
            "bacteria_pop",
            NodeKind::Organism,
            true,
        ),
        NodeSpec::new(NodeId::Toxin, "toxin", NodeKind::Chemical, true),
        NodeSpec::new(NodeId::Nutrient, "nutrient", NodeKind::Chemical, true),
        NodeSpec::new(NodeId::Enzyme, "enzyme", NodeKind::Latent, false),
    ]
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum EdgeSign {
    Positive,
    Negative,
}

impl EdgeSign {
    pub fn from_weight(weight: f32) -> Self {
        if weight >= 0.0 {
            Self::Positive
        } else {
            Self::Negative
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct EdgeSpec {
    pub from: NodeId,
    pub to: NodeId,
    pub weight: f32,
}

impl EdgeSpec {
    pub fn sign(&self) -> EdgeSign {
        EdgeSign::from_weight(self.weight)
    }
}
