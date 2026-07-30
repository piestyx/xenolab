use serde::{Deserialize, Serialize};

use crate::engine::contamination::ContaminationLevel;
use crate::engine::ids::{NodeId, ObjectiveId};
use crate::engine::notebook::Hypothesis;
use crate::engine::publication::{Publication, PUBLICATION_LIMIT};
use crate::engine::world::WorldState;

pub const ACTION_LIMIT: u32 = 30;
pub const OBJECTIVE_HOLD_REQUIRED: u32 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunFailure {
    ActionBudgetExhausted,
    ContainmentLost,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Active,
    Won,
    Failed(RunFailure),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectiveProgress {
    pub current: u32,
    pub required: u32,
}

impl ObjectiveProgress {
    pub fn new() -> Self {
        Self {
            current: 0,
            required: OBJECTIVE_HOLD_REQUIRED,
        }
    }

    pub fn is_complete(self) -> bool {
        self.current >= self.required
    }
}

impl Default for ObjectiveProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunState {
    pub seed: u64,
    pub objective: ObjectiveId,
    pub status: RunStatus,
    pub action_limit: u32,
    pub actions_used: u32,
    pub objective_progress: ObjectiveProgress,
}

impl RunState {
    pub fn new(seed: u64, objective: ObjectiveId) -> Self {
        Self::with_action_limit(seed, objective, ACTION_LIMIT)
    }

    pub(crate) fn with_action_limit(seed: u64, objective: ObjectiveId, action_limit: u32) -> Self {
        Self {
            seed,
            objective,
            status: RunStatus::Active,
            action_limit,
            actions_used: 0,
            objective_progress: ObjectiveProgress::new(),
        }
    }

    pub fn actions_remaining(&self) -> u32 {
        self.action_limit.saturating_sub(self.actions_used)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunDebrief {
    pub seed: u64,
    pub objective: ObjectiveId,
    pub outcome: RunStatus,
    pub failure_reason: Option<RunFailure>,
    pub actions_used: u32,
    pub action_limit: u32,
    pub final_tick: u32,
    pub final_contamination: f32,
    pub final_contamination_level: ContaminationLevel,
    pub peak_contamination: f32,
    pub compromised_scans: u32,
    pub critical_scans: u32,
    pub final_plant: f32,
    pub final_fungus: f32,
    pub final_bacteria: f32,
    pub final_toxin: f32,
    pub final_nutrient: f32,
    pub objective_progress: ObjectiveProgress,
    pub event_hash: String,
    pub notebook: Vec<Hypothesis>,
    pub publications: Vec<Publication>,
    pub research_credits: u32,
    pub publications_used: u32,
    pub publication_limit: u32,
}

impl RunDebrief {
    #[allow(clippy::too_many_arguments)]
    pub fn from_terminal_state(
        run: &RunState,
        state: &WorldState,
        tick: u32,
        contamination: f32,
        peak_contamination: f32,
        compromised_scans: u32,
        critical_scans: u32,
        event_hash: String,
        notebook: Vec<Hypothesis>,
        publications: Vec<Publication>,
        research_credits: u32,
    ) -> Self {
        Self {
            seed: run.seed,
            objective: run.objective,
            outcome: run.status,
            failure_reason: match run.status {
                RunStatus::Failed(reason) => Some(reason),
                RunStatus::Active | RunStatus::Won => None,
            },
            actions_used: run.actions_used,
            action_limit: run.action_limit,
            final_tick: tick,
            final_contamination: contamination,
            final_contamination_level: ContaminationLevel::from_value(contamination),
            peak_contamination,
            compromised_scans,
            critical_scans,
            final_plant: state.get(NodeId::PlantPop),
            final_fungus: state.get(NodeId::FungusLoad),
            final_bacteria: state.get(NodeId::BacteriaPop),
            final_toxin: state.get(NodeId::Toxin),
            final_nutrient: state.get(NodeId::Nutrient),
            objective_progress: run.objective_progress,
            event_hash,
            notebook,
            publications_used: publications.len() as u32,
            publication_limit: PUBLICATION_LIMIT,
            publications,
            research_credits,
        }
    }
}
