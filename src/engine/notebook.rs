use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::engine::ids::NodeId;
use crate::engine::world::WorldState;

pub const NOTEBOOK_CAPACITY: usize = 8;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HypothesisDirection {
    Increases,
    Decreases,
}

impl HypothesisDirection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Increases => "increases",
            Self::Decreases => "decreases",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ObservableVariable {
    PlantPopulation,
    FungusPopulation,
    BacteriaPopulation,
    ToxinConcentration,
    NutrientConcentration,
    UvLevel,
}

impl ObservableVariable {
    pub const ALL: [Self; 6] = [
        Self::PlantPopulation,
        Self::FungusPopulation,
        Self::BacteriaPopulation,
        Self::ToxinConcentration,
        Self::NutrientConcentration,
        Self::UvLevel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PlantPopulation => "Plant population",
            Self::FungusPopulation => "Fungus population",
            Self::BacteriaPopulation => "Bacteria population",
            Self::ToxinConcentration => "Toxin concentration",
            Self::NutrientConcentration => "Nutrient concentration",
            Self::UvLevel => "UV level",
        }
    }

    pub fn node(self) -> NodeId {
        match self {
            Self::PlantPopulation => NodeId::PlantPop,
            Self::FungusPopulation => NodeId::FungusLoad,
            Self::BacteriaPopulation => NodeId::BacteriaPop,
            Self::ToxinConcentration => NodeId::Toxin,
            Self::NutrientConcentration => NodeId::Nutrient,
            Self::UvLevel => NodeId::UvLevel,
        }
    }

    pub fn value(self, state: &WorldState) -> f32 {
        state.get(self.node())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HypothesisId(pub u32);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hypothesis {
    pub id: HypothesisId,
    pub cause: ObservableVariable,
    pub effect: ObservableVariable,
    pub direction: HypothesisDirection,
}

impl Hypothesis {
    pub fn sentence(self) -> String {
        format!(
            "{} {} {}",
            self.cause.label(),
            self.direction.label(),
            self.effect.label()
        )
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum NotebookError {
    #[error("cause and effect must be different variables")]
    SameVariable,
    #[error("that hypothesis is already recorded")]
    DuplicateHypothesis,
    #[error("Notebook is full")]
    NotebookFull,
    #[error("run has already resolved")]
    RunResolved,
    #[error("hypothesis was not found")]
    HypothesisNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notebook {
    hypotheses: Vec<Hypothesis>,
    next_id: u32,
}

impl Notebook {
    pub fn new() -> Self {
        Self {
            hypotheses: Vec::new(),
            next_id: 1,
        }
    }

    pub fn hypotheses(&self) -> &[Hypothesis] {
        &self.hypotheses
    }

    pub fn capacity(&self) -> usize {
        NOTEBOOK_CAPACITY
    }

    pub fn remaining_slots(&self) -> usize {
        NOTEBOOK_CAPACITY.saturating_sub(self.hypotheses.len())
    }

    pub fn next_hypothesis_id(&self) -> HypothesisId {
        HypothesisId(self.next_id)
    }

    pub(crate) fn add(
        &mut self,
        cause: ObservableVariable,
        direction: HypothesisDirection,
        effect: ObservableVariable,
    ) -> Result<HypothesisId, NotebookError> {
        validate_pair(&self.hypotheses, None, cause, direction, effect)?;
        if self.hypotheses.len() >= NOTEBOOK_CAPACITY {
            return Err(NotebookError::NotebookFull);
        }
        let id = HypothesisId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.hypotheses.push(Hypothesis {
            id,
            cause,
            effect,
            direction,
        });
        Ok(id)
    }

    pub(crate) fn edit(
        &mut self,
        id: HypothesisId,
        cause: ObservableVariable,
        direction: HypothesisDirection,
        effect: ObservableVariable,
    ) -> Result<(), NotebookError> {
        let index = self
            .hypotheses
            .iter()
            .position(|hypothesis| hypothesis.id == id)
            .ok_or(NotebookError::HypothesisNotFound)?;
        validate_pair(&self.hypotheses, Some(id), cause, direction, effect)?;
        self.hypotheses[index] = Hypothesis {
            id,
            cause,
            effect,
            direction,
        };
        Ok(())
    }

    pub(crate) fn remove(&mut self, id: HypothesisId) -> Result<(), NotebookError> {
        let index = self
            .hypotheses
            .iter()
            .position(|hypothesis| hypothesis.id == id)
            .ok_or(NotebookError::HypothesisNotFound)?;
        self.hypotheses.remove(index);
        Ok(())
    }
}

impl Default for Notebook {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_pair(
    hypotheses: &[Hypothesis],
    editing: Option<HypothesisId>,
    cause: ObservableVariable,
    direction: HypothesisDirection,
    effect: ObservableVariable,
) -> Result<(), NotebookError> {
    if cause == effect {
        return Err(NotebookError::SameVariable);
    }
    if hypotheses.iter().any(|hypothesis| {
        Some(hypothesis.id) != editing
            && hypothesis.cause == cause
            && hypothesis.direction == direction
            && hypothesis.effect == effect
    }) {
        return Err(NotebookError::DuplicateHypothesis);
    }
    Ok(())
}
