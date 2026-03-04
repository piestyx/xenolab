use serde::{Deserialize, Serialize};

pub const NODE_COUNT: usize = 7;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    UvLevel,
    PlantPop,
    FungusLoad,
    BacteriaPop,
    Toxin,
    Nutrient,
    Enzyme,
}

impl NodeId {
    pub const ALL: [Self; NODE_COUNT] = [
        Self::UvLevel,
        Self::PlantPop,
        Self::FungusLoad,
        Self::BacteriaPop,
        Self::Toxin,
        Self::Nutrient,
        Self::Enzyme,
    ];

    pub fn as_index(self) -> usize {
        match self {
            Self::UvLevel => 0,
            Self::PlantPop => 1,
            Self::FungusLoad => 2,
            Self::BacteriaPop => 3,
            Self::Toxin => 4,
            Self::Nutrient => 5,
            Self::Enzyme => 6,
        }
    }

    pub fn stable_name(self) -> &'static str {
        match self {
            Self::UvLevel => "uv_level",
            Self::PlantPop => "plant_pop",
            Self::FungusLoad => "fungus_load",
            Self::BacteriaPop => "bacteria_pop",
            Self::Toxin => "toxin",
            Self::Nutrient => "nutrient",
            Self::Enzyme => "enzyme",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObjectiveId {
    StabilizePlant,
    Detox,
    PreventCollapse,
}

impl ObjectiveId {
    pub fn for_seed(seed: u64) -> Self {
        match seed % 3 {
            0 => Self::StabilizePlant,
            1 => Self::Detox,
            _ => Self::PreventCollapse,
        }
    }
}
