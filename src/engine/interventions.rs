use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Intervention {
    SetUvLow,
    SetUvHigh,
    AddNutrient(f32),
    AddToxin(f32),
    NeutralizeToxin(f32),
    RemoveFungus,
    RemoveBacteria,
    SterilizeSample,
    ScanPopulation,
    ScanChemicals,
    AdvanceTime,
}

impl Intervention {
    pub const DEFAULT_DELTA: f32 = 20.0;

    pub fn add_nutrient_default() -> Self {
        Self::AddNutrient(Self::DEFAULT_DELTA)
    }

    pub fn add_toxin_default() -> Self {
        Self::AddToxin(Self::DEFAULT_DELTA)
    }

    pub fn neutralize_toxin_default() -> Self {
        Self::NeutralizeToxin(Self::DEFAULT_DELTA)
    }

    pub fn ticks_time(&self) -> bool {
        !matches!(self, Self::ScanPopulation | Self::ScanChemicals)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::SetUvLow => "Set UV Low",
            Self::SetUvHigh => "Set UV High",
            Self::AddNutrient(_) => "Add Nutrient",
            Self::AddToxin(_) => "Add Toxin",
            Self::NeutralizeToxin(_) => "Neutralize Toxin",
            Self::RemoveFungus => "Remove Fungus",
            Self::RemoveBacteria => "Remove Bacteria",
            Self::SterilizeSample => "Sterilize Sample",
            Self::ScanPopulation => "Scan Population",
            Self::ScanChemicals => "Scan Chemicals",
            Self::AdvanceTime => "Advance Time",
        }
    }
}
