use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::engine::interventions::Intervention;
use crate::engine::notebook::{
    HypothesisDirection, HypothesisId, NotebookError, ObservableVariable,
};
use crate::engine::publication::PublicationError;
use crate::engine::repair::{RepairError, RepairTrack};
use crate::engine::sim::{SimError, Simulator};
use crate::worldgen::generate_playable;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypothesisDraft {
    pub cause: ObservableVariable,
    pub direction: HypothesisDirection,
    pub effect: ObservableVariable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplayOperation {
    Apply(Intervention),
    NotebookAdd(HypothesisDraft),
    NotebookEdit {
        id: HypothesisId,
        replacement: HypothesisDraft,
    },
    NotebookRemove(HypothesisId),
    Publish(HypothesisId),
    PurchaseRepair(RepairTrack),
    RestartSameSeed,
    RestartNewSeed(u64),
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("replay operation {index} failed: simulation error: {source}")]
    Simulation { index: usize, source: SimError },
    #[error("replay operation {index} failed: Notebook error: {source}")]
    Notebook { index: usize, source: NotebookError },
    #[error("replay operation {index} failed: publication error: {source}")]
    Publication {
        index: usize,
        source: PublicationError,
    },
    #[error("replay operation {index} failed: repair error: {source}")]
    Repair { index: usize, source: RepairError },
}

pub fn replay(seed: u64, operations: &[ReplayOperation]) -> Result<Simulator, ReplayError> {
    let mut current_seed = seed;
    let mut simulator = Simulator::new(generate_playable(current_seed));

    for (index, operation) in operations.iter().enumerate() {
        let result = match operation {
            ReplayOperation::Apply(action) => simulator
                .apply(action.clone())
                .map(|_| ())
                .map_err(ReplayApplyError::Simulation),
            ReplayOperation::NotebookAdd(draft) => simulator
                .add_hypothesis(draft.cause, draft.direction, draft.effect)
                .map(|_| ())
                .map_err(ReplayApplyError::Notebook),
            ReplayOperation::NotebookEdit { id, replacement } => simulator
                .edit_hypothesis(
                    *id,
                    replacement.cause,
                    replacement.direction,
                    replacement.effect,
                )
                .map_err(ReplayApplyError::Notebook),
            ReplayOperation::NotebookRemove(id) => simulator
                .remove_hypothesis(*id)
                .map_err(ReplayApplyError::Notebook),
            ReplayOperation::Publish(id) => simulator
                .publish_hypothesis(*id)
                .map(|_| ())
                .map_err(ReplayApplyError::Publication),
            ReplayOperation::PurchaseRepair(track) => simulator
                .purchase_repair(*track)
                .map(|_| ())
                .map_err(ReplayApplyError::Repair),
            ReplayOperation::RestartSameSeed => {
                simulator = Simulator::new(generate_playable(current_seed));
                Ok(())
            }
            ReplayOperation::RestartNewSeed(seed) => {
                current_seed = *seed;
                simulator = Simulator::new(generate_playable(current_seed));
                Ok(())
            }
        };

        match result {
            Ok(()) => {}
            Err(ReplayApplyError::Simulation(source)) => {
                return Err(ReplayError::Simulation { index, source });
            }
            Err(ReplayApplyError::Notebook(source)) => {
                return Err(ReplayError::Notebook { index, source });
            }
            Err(ReplayApplyError::Publication(source)) => {
                return Err(ReplayError::Publication { index, source });
            }
            Err(ReplayApplyError::Repair(source)) => {
                return Err(ReplayError::Repair { index, source });
            }
        }
    }
    Ok(simulator)
}

enum ReplayApplyError {
    Simulation(SimError),
    Notebook(NotebookError),
    Publication(PublicationError),
    Repair(RepairError),
}

impl From<SimError> for ReplayApplyError {
    fn from(error: SimError) -> Self {
        Self::Simulation(error)
    }
}

impl From<NotebookError> for ReplayApplyError {
    fn from(error: NotebookError) -> Self {
        Self::Notebook(error)
    }
}

impl From<PublicationError> for ReplayApplyError {
    fn from(error: PublicationError) -> Self {
        Self::Publication(error)
    }
}

impl From<RepairError> for ReplayApplyError {
    fn from(error: RepairError) -> Self {
        Self::Repair(error)
    }
}
