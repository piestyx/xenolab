use serde::Serialize;

use crate::engine::ids::NodeId;

pub const INFLUENCE_SCALE: f32 = 8.0;
pub const STABILITY_CAP: f32 = 2.0;
pub const ACCEPTANCE_ATTEMPTS: u32 = 32;

pub const MIN_POS_WEIGHT: f32 = 0.4;
pub const MAX_POS_WEIGHT: f32 = 1.2;
pub const MIN_NEG_WEIGHT: f32 = -1.2;
pub const MAX_NEG_WEIGHT: f32 = -0.4;
pub const MIN_TWIST_MAGNITUDE: f32 = 0.3;
pub const MAX_TWIST_MAGNITUDE: f32 = 0.9;

pub const MIN_BIAS: f32 = -0.2;
pub const MAX_BIAS: f32 = 0.2;

pub const SIGMA_ORGANISM: f32 = 0.5;
pub const SIGMA_CHEMICAL: f32 = 0.8;
pub const SIGMA_LATENT: f32 = 0.6;
pub const SIGMA_ENV: f32 = 0.0;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum MenuPolarity {
    Positive,
    Negative,
    Either,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct EdgeMenuItem {
    pub id: &'static str,
    pub from: NodeId,
    pub to: NodeId,
    pub polarity: MenuPolarity,
    pub is_twist: bool,
}

pub const MANDATORY_EDGES: [EdgeMenuItem; 5] = [
    EdgeMenuItem {
        id: "fungus_enzyme",
        from: NodeId::FungusLoad,
        to: NodeId::Enzyme,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "uv_enzyme",
        from: NodeId::UvLevel,
        to: NodeId::Enzyme,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "enzyme_plant",
        from: NodeId::Enzyme,
        to: NodeId::PlantPop,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "toxin_bacteria",
        from: NodeId::Toxin,
        to: NodeId::BacteriaPop,
        polarity: MenuPolarity::Negative,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "toxin_plant_hazard",
        from: NodeId::Toxin,
        to: NodeId::PlantPop,
        polarity: MenuPolarity::Negative,
        is_twist: false,
    },
];

pub const FEEDBACK_MENU: [EdgeMenuItem; 3] = [
    EdgeMenuItem {
        id: "bacteria_toxin_decay",
        from: NodeId::BacteriaPop,
        to: NodeId::Toxin,
        polarity: MenuPolarity::Negative,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "plant_nutrient_deplete",
        from: NodeId::PlantPop,
        to: NodeId::Nutrient,
        polarity: MenuPolarity::Negative,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "fungus_toxin_prod",
        from: NodeId::FungusLoad,
        to: NodeId::Toxin,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
];

pub const CONFOUNDER_MENU: [EdgeMenuItem; 3] = [
    EdgeMenuItem {
        id: "nutrient_plant",
        from: NodeId::Nutrient,
        to: NodeId::PlantPop,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "uv_toxin",
        from: NodeId::UvLevel,
        to: NodeId::Toxin,
        polarity: MenuPolarity::Either,
        is_twist: false,
    },
    EdgeMenuItem {
        id: "bacteria_nutrient",
        from: NodeId::BacteriaPop,
        to: NodeId::Nutrient,
        polarity: MenuPolarity::Positive,
        is_twist: false,
    },
];

pub const TWIST_MENU: [EdgeMenuItem; 2] = [
    EdgeMenuItem {
        id: "toxin_fungus",
        from: NodeId::Toxin,
        to: NodeId::FungusLoad,
        polarity: MenuPolarity::Either,
        is_twist: true,
    },
    EdgeMenuItem {
        id: "nutrient_fungus",
        from: NodeId::Nutrient,
        to: NodeId::FungusLoad,
        polarity: MenuPolarity::Positive,
        is_twist: true,
    },
];
