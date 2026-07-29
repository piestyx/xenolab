use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

use crate::engine::contamination::{ContaminationLevel, CONTAINMENT_LOST_THRESHOLD};
use crate::engine::graph::Graph;
use crate::engine::ids::NodeId;
use crate::engine::interventions::Intervention;
use crate::engine::math;
use crate::engine::measurement::{sample_standard_normal, scan_chemicals, scan_population};
use crate::engine::notebook::{
    HypothesisDirection, HypothesisId, Notebook, NotebookError, ObservableVariable,
};
use crate::engine::run::{RunDebrief, RunFailure, RunState, RunStatus, ACTION_LIMIT};
use crate::engine::runlog::{RunEvent, RunLog};
use crate::engine::world::{UvToxinThresholdMode, WorldRecipe, WorldState};

const INFLUENCE_SCALE: f32 = crate::worldgen::spec::INFLUENCE_SCALE;
const ORG_MAINTENANCE: f32 = 1.0;
const ORG_CAPACITY_K: f32 = 0.025;
const TOXIN_BASE_DECAY: f32 = 0.4;
const NUTRIENT_BASE_DECAY: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseMode {
    Normal,
    Disabled,
}

#[derive(Debug, Error)]
pub enum SimError {
    #[error("run has already resolved")]
    RunResolved,
    #[error("delta value for {0} must be finite and >= 0")]
    InvalidDelta(&'static str),
    #[error("invalid edge index from={from} to={to}")]
    InvalidEdgeIndex { from: usize, to: usize },
    #[error("invalid state value at node {node}: {value}")]
    InvalidStateValue { node: String, value: f32 },
}

pub struct Simulator {
    recipe: WorldRecipe,
    graph: Graph,
    state: WorldState,
    tick: u32,
    contamination: f32,
    peak_contamination: f32,
    compromised_scans: u32,
    critical_scans: u32,
    noise_mode: NoiseMode,
    rng: ChaCha8Rng,
    runlog: RunLog,
    notebook: Notebook,
    run: RunState,
    debrief: Option<RunDebrief>,
    lifecycle_enabled: bool,
}

impl Simulator {
    pub fn new(recipe: WorldRecipe) -> Self {
        Self::with_config(recipe, ACTION_LIMIT, NoiseMode::Normal, true)
    }

    pub fn new_no_noise(recipe: WorldRecipe) -> Self {
        Self::with_config(recipe, ACTION_LIMIT, NoiseMode::Disabled, true)
    }

    pub fn new_for_analysis(recipe: WorldRecipe) -> Self {
        Self::with_config(recipe, u32::MAX, NoiseMode::Normal, false)
    }

    pub fn new_no_noise_for_analysis(recipe: WorldRecipe) -> Self {
        Self::with_config(recipe, u32::MAX, NoiseMode::Disabled, false)
    }

    fn with_config(
        recipe: WorldRecipe,
        action_limit: u32,
        noise_mode: NoiseMode,
        lifecycle_enabled: bool,
    ) -> Self {
        let seed = recipe.seed;
        let objective = recipe.objective;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&recipe.seed.to_le_bytes());
        hasher.update(&recipe.attempt.to_le_bytes());
        hasher.update(b"sim");
        let hash = hasher.finalize();
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&hash.as_bytes()[..8]);
        let rng_seed = u64::from_le_bytes(bytes);

        Self {
            graph: Graph::new(recipe.edges.clone()),
            state: recipe.initial_state,
            recipe,
            tick: 0,
            contamination: 0.0,
            peak_contamination: 0.0,
            compromised_scans: 0,
            critical_scans: 0,
            noise_mode,
            rng: ChaCha8Rng::seed_from_u64(rng_seed),
            runlog: RunLog::default(),
            notebook: Notebook::new(),
            run: RunState::with_action_limit(seed, objective, action_limit),
            debrief: None,
            lifecycle_enabled,
        }
    }

    pub fn with_noise_mode(recipe: WorldRecipe, noise_mode: NoiseMode) -> Self {
        Self::with_config(recipe, ACTION_LIMIT, noise_mode, true)
    }

    pub fn state(&self) -> &WorldState {
        &self.state
    }

    pub fn tick_index(&self) -> u32 {
        self.tick
    }

    pub fn contamination(&self) -> f32 {
        self.contamination
    }

    pub fn contamination_level(&self) -> ContaminationLevel {
        ContaminationLevel::from_value(self.contamination)
    }

    pub fn peak_contamination(&self) -> f32 {
        self.peak_contamination
    }

    pub fn compromised_scans(&self) -> u32 {
        self.compromised_scans
    }

    pub fn critical_scans(&self) -> u32 {
        self.critical_scans
    }

    pub fn events(&self) -> &[RunEvent] {
        &self.runlog.events
    }

    pub fn run_state(&self) -> &RunState {
        &self.run
    }

    pub fn debrief(&self) -> Option<&RunDebrief> {
        self.debrief.as_ref()
    }

    pub fn notebook(&self) -> &Notebook {
        &self.notebook
    }

    pub fn add_hypothesis(
        &mut self,
        cause: ObservableVariable,
        direction: HypothesisDirection,
        effect: ObservableVariable,
    ) -> Result<HypothesisId, NotebookError> {
        self.ensure_notebook_editable()?;
        self.notebook.add(cause, direction, effect)
    }

    pub fn edit_hypothesis(
        &mut self,
        id: HypothesisId,
        cause: ObservableVariable,
        direction: HypothesisDirection,
        effect: ObservableVariable,
    ) -> Result<(), NotebookError> {
        self.ensure_notebook_editable()?;
        self.notebook.edit(id, cause, direction, effect)
    }

    pub fn remove_hypothesis(&mut self, id: HypothesisId) -> Result<(), NotebookError> {
        self.ensure_notebook_editable()?;
        self.notebook.remove(id)
    }

    fn ensure_notebook_editable(&self) -> Result<(), NotebookError> {
        if self.lifecycle_enabled && self.run.status != RunStatus::Active {
            Err(NotebookError::RunResolved)
        } else {
            Ok(())
        }
    }

    pub fn apply(
        &mut self,
        action: crate::engine::interventions::Intervention,
    ) -> Result<crate::engine::runlog::RunEvent, crate::engine::sim::SimError> {
        if self.lifecycle_enabled && self.run.status != RunStatus::Active {
            return Err(SimError::RunResolved);
        }

        let scan_level = if matches!(
            &action,
            Intervention::ScanPopulation | Intervention::ScanChemicals
        ) {
            Some(self.contamination_level())
        } else {
            None
        };
        let mut measurements = Vec::new();
        self.apply_intervention(&action, &mut measurements)?;
        if action.ticks_time() {
            self.tick_once()?;
        }

        self.add_contamination(action.contamination_cost());
        if let Some(level) = scan_level {
            match level {
                ContaminationLevel::Compromised => self.compromised_scans += 1,
                ContaminationLevel::Critical => self.critical_scans += 1,
                ContaminationLevel::Stable | ContaminationLevel::Lost => {}
            }
        }

        let event = RunEvent {
            tick_index: self.tick,
            intervention: action,
            measurements,
            state_snapshot: self.state,
            contamination: self.contamination,
        };
        self.runlog.push(event.clone());
        if self.lifecycle_enabled {
            self.run.actions_used += 1;
            self.update_objective_progress();
            if self.run.objective_progress.is_complete() {
                self.run.status = RunStatus::Won;
            } else if self.contamination >= CONTAINMENT_LOST_THRESHOLD {
                self.run.status = RunStatus::Failed(RunFailure::ContainmentLost);
            } else if self.run.actions_used >= self.run.action_limit {
                self.run.status = RunStatus::Failed(RunFailure::ActionBudgetExhausted);
            }
            if self.run.status != RunStatus::Active {
                self.debrief = Some(self.build_debrief());
            }
        }
        Ok(event)
    }

    fn update_objective_progress(&mut self) {
        let qualifies = match self.run.objective {
            crate::engine::ids::ObjectiveId::StabilizePlant => {
                self.state.get(NodeId::PlantPop) >= 60.0
            }
            crate::engine::ids::ObjectiveId::Detox => self.state.get(NodeId::Toxin) <= 15.0,
            crate::engine::ids::ObjectiveId::PreventCollapse => {
                self.state.get(NodeId::PlantPop) >= 25.0
                    && self.state.get(NodeId::BacteriaPop) >= 25.0
            }
        };

        if qualifies {
            self.run.objective_progress.current = self
                .run
                .objective_progress
                .current
                .saturating_add(1)
                .min(self.run.objective_progress.required);
        } else {
            self.run.objective_progress.current = 0;
        }
    }

    fn build_debrief(&self) -> RunDebrief {
        RunDebrief::from_terminal_state(
            &self.run,
            &self.state,
            self.tick,
            self.contamination,
            self.peak_contamination,
            self.compromised_scans,
            self.critical_scans,
            crate::engine::runlog::hash_events(&self.runlog.events)
                .to_hex()
                .to_string(),
            self.notebook.hypotheses().to_vec(),
        )
    }

    fn add_contamination(&mut self, cost: u32) {
        let delta = cost as f32;
        if delta == 0.0 {
            return;
        }

        self.contamination = if self.contamination >= f32::MAX - delta {
            f32::MAX
        } else {
            self.contamination + delta
        };
        self.peak_contamination = self.peak_contamination.max(self.contamination);
    }

    fn apply_intervention(
        &mut self,
        action: &Intervention,
        measurements: &mut Vec<crate::engine::measurement::MeasurementRecord>,
    ) -> Result<(), SimError> {
        match action {
            Intervention::SetUvLow => {
                self.state.set(NodeId::UvLevel, 0.0);
            }
            Intervention::SetUvHigh => {
                self.state.set(NodeId::UvLevel, 100.0);
            }
            Intervention::AddNutrient(delta) => {
                validate_non_negative(*delta, "AddNutrient")?;
                let next = self.state.get(NodeId::Nutrient) + delta;
                self.state.set(NodeId::Nutrient, next);
            }
            Intervention::AddToxin(delta) => {
                validate_non_negative(*delta, "AddToxin")?;
                let next = self.state.get(NodeId::Toxin) + delta;
                self.state.set(NodeId::Toxin, next);
            }
            Intervention::NeutralizeToxin(delta) => {
                validate_non_negative(*delta, "NeutralizeToxin")?;
                let next = self.state.get(NodeId::Toxin) - delta;
                self.state.set(NodeId::Toxin, next.max(0.0));
            }
            Intervention::RemoveFungus => {
                self.state.set(NodeId::FungusLoad, 0.0);
            }
            Intervention::RemoveBacteria => {
                self.state.set(NodeId::BacteriaPop, 0.0);
            }
            Intervention::SterilizeSample => {
                let next = self.state.get(NodeId::FungusLoad) - 50.0;
                self.state.set(NodeId::FungusLoad, next);
            }
            Intervention::ScanPopulation => {
                measurements.extend(scan_population(
                    &self.state,
                    &mut self.rng,
                    self.tick,
                    self.contamination,
                ));
            }
            Intervention::ScanChemicals => {
                measurements.extend(scan_chemicals(
                    &self.state,
                    &mut self.rng,
                    self.tick,
                    self.contamination,
                ));
            }
            Intervention::AdvanceTime => {}
        }
        Ok(())
    }

    fn tick_once(&mut self) -> Result<(), SimError> {
        let mut next = self.state;
        let update_order = [
            NodeId::Enzyme,
            NodeId::Toxin,
            NodeId::Nutrient,
            NodeId::PlantPop,
            NodeId::FungusLoad,
            NodeId::BacteriaPop,
        ];

        for node in update_order {
            let current = self.require_state_value(node, next.get(node))?;
            let bias = self.recipe.biases[node.as_index()];
            let incoming = self.incoming_pairs(node, &next)?;
            let influence = math::compute_influence(&incoming, &next.values);
            let sigma = self.recipe.noise_sigma[node.as_index()];
            let noise = if self.noise_mode == NoiseMode::Disabled {
                0.0
            } else if sigma > 0.0 {
                sample_standard_normal(&mut self.rng) * sigma
            } else {
                0.0
            };
            let delta = bias + influence * INFLUENCE_SCALE + noise;
            let updated = if is_organism(node) {
                let raw =
                    math::apply_organism_dynamics(current, delta, ORG_MAINTENANCE, ORG_CAPACITY_K);
                math::clamp01(raw)
            } else if is_chemical(node) {
                let raw = current + delta;
                let decay = match node {
                    NodeId::Toxin => TOXIN_BASE_DECAY,
                    NodeId::Nutrient => NUTRIENT_BASE_DECAY,
                    _ => 0.0,
                };
                let mut chemical = math::apply_base_decay(raw, decay);
                if node == NodeId::Toxin {
                    chemical = self.apply_uv_toxin_threshold(chemical, next.get(NodeId::UvLevel));
                }
                math::clamp01(chemical)
            } else {
                math::apply_update(current, bias, influence, INFLUENCE_SCALE, noise)
            };
            let validated = self.require_state_value(node, updated)?;
            next.set(node, validated);
        }

        // Keep UV constrained to the discrete intervention-controlled levels.
        let uv = self.state.get(NodeId::UvLevel);
        next.set(NodeId::UvLevel, uv);

        self.state = next;
        self.tick = self.tick.saturating_add(1);
        Ok(())
    }

    fn incoming_pairs(
        &self,
        target: NodeId,
        state: &WorldState,
    ) -> Result<Vec<(usize, f32)>, SimError> {
        let mut pairs = Vec::new();
        let values_len = state.values.len();
        let to_idx = target.as_index();

        if to_idx >= values_len {
            return Err(SimError::InvalidEdgeIndex {
                from: to_idx,
                to: to_idx,
            });
        }

        for edge in self.graph.incoming(target) {
            let from_idx = edge.from.as_index();
            if from_idx >= values_len {
                return Err(SimError::InvalidEdgeIndex {
                    from: from_idx,
                    to: to_idx,
                });
            }
            pairs.push((from_idx, edge.weight));
        }

        Ok(pairs)
    }

    fn require_state_value(&self, node: NodeId, value: f32) -> Result<f32, SimError> {
        if math::is_valid_state_value(value) {
            Ok(value)
        } else {
            Err(SimError::InvalidStateValue {
                node: node.stable_name().to_string(),
                value,
            })
        }
    }

    fn apply_uv_toxin_threshold(&self, toxin_value: f32, uv_level: f32) -> f32 {
        let threshold = self.recipe.threshold;
        if uv_level < threshold.uv_cutoff {
            return toxin_value;
        }

        match threshold.uv_toxin_mode {
            UvToxinThresholdMode::None => toxin_value,
            UvToxinThresholdMode::Burn => toxin_value - threshold.toxin_delta,
            UvToxinThresholdMode::Create => toxin_value + threshold.toxin_delta,
        }
    }
}

fn is_organism(node: NodeId) -> bool {
    matches!(
        node,
        NodeId::PlantPop | NodeId::FungusLoad | NodeId::BacteriaPop
    )
}

fn is_chemical(node: NodeId) -> bool {
    matches!(node, NodeId::Toxin | NodeId::Nutrient)
}

fn validate_non_negative(delta: f32, label: &'static str) -> Result<(), SimError> {
    if delta.is_finite() && delta >= 0.0 {
        Ok(())
    } else {
        Err(SimError::InvalidDelta(label))
    }
}
