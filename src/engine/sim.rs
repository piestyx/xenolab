use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

use crate::engine::contamination::{ContaminationLevel, CONTAINMENT_LOST_THRESHOLD};
use crate::engine::graph::Graph;
use crate::engine::ids::NodeId;
use crate::engine::interventions::Intervention;
use crate::engine::math;
use crate::engine::measurement::{
    sample_standard_normal, scan_chemicals_with_calibration, scan_population_with_calibration,
};
use crate::engine::notebook::{
    HypothesisDirection, HypothesisId, Notebook, NotebookError, ObservableVariable,
};
use crate::engine::publication::{
    evaluate, Publication, PublicationError, MAX_RESEARCH_CREDITS, PUBLICATION_LIMIT,
};
use crate::engine::repair::{
    CalibrationLevel, ContainmentLevel, CreditWallet, RepairError, RepairPurchase,
    RepairPurchaseId, RepairTrack,
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
    publications: Vec<Publication>,
    next_publication_id: u32,
    credit_wallet: CreditWallet,
    calibration_level: CalibrationLevel,
    containment_level: ContainmentLevel,
    repair_purchases: Vec<RepairPurchase>,
    next_repair_purchase_id: u32,
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
            publications: Vec::new(),
            next_publication_id: 1,
            credit_wallet: CreditWallet::new(),
            calibration_level: CalibrationLevel::Level0,
            containment_level: ContainmentLevel::Level0,
            repair_purchases: Vec::new(),
            next_repair_purchase_id: 1,
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

    pub fn publications(&self) -> &[Publication] {
        &self.publications
    }

    pub fn research_credits(&self) -> u32 {
        self.credit_wallet.available()
    }

    pub fn max_research_credits(&self) -> u32 {
        MAX_RESEARCH_CREDITS
    }

    pub fn credits_earned(&self) -> u32 {
        self.credit_wallet.earned()
    }

    pub fn credits_spent(&self) -> u32 {
        self.credit_wallet.spent()
    }

    pub fn credits_available(&self) -> u32 {
        self.credit_wallet.available()
    }

    pub fn calibration_level(&self) -> CalibrationLevel {
        self.calibration_level
    }

    pub fn containment_level(&self) -> ContainmentLevel {
        self.containment_level
    }

    pub fn repair_purchases(&self) -> &[RepairPurchase] {
        &self.repair_purchases
    }

    pub fn calibration_multiplier(&self) -> f32 {
        self.calibration_level.noise_multiplier()
    }

    pub fn containment_reduction(&self) -> u32 {
        self.containment_level.contamination_reduction()
    }

    pub fn effective_contamination_cost(&self, action: &Intervention) -> u32 {
        action
            .contamination_cost()
            .saturating_sub(self.containment_reduction())
    }

    /// Hashes the complete outcome-relevant run record.
    ///
    /// This intentionally remains separate from `hash_events`, which is the
    /// gameplay-event hash and excludes Notebook/publication/repair records.
    pub fn verification_hash(&self) -> blake3::Hash {
        let recipe_hash = self.recipe.recipe_hash().ok();
        let event_hash = crate::engine::runlog::hash_events(&self.runlog.events);
        let payload = (
            "xenolab-verification-v1",
            self.recipe.seed,
            recipe_hash.map(|hash| hash.as_bytes().to_vec()),
            event_hash.as_bytes().to_vec(),
            &self.notebook,
            &self.publications,
            self.credit_wallet.earned(),
            self.credit_wallet.spent(),
            self.credit_wallet.available(),
            self.calibration_level,
            self.containment_level,
            &self.repair_purchases,
            &self.run,
            &self.debrief,
        );
        match serde_json::to_vec(&payload) {
            Ok(bytes) => blake3::hash(&bytes),
            Err(_) => blake3::hash(b"xenolab-verification-error"),
        }
    }

    pub fn purchase_repair(&mut self, track: RepairTrack) -> Result<RepairPurchase, RepairError> {
        if self.lifecycle_enabled && self.run.status != RunStatus::Active {
            return Err(RepairError::RunResolved);
        }
        let (level_before, cost) = match track {
            RepairTrack::Calibration => (
                self.calibration_level.level(),
                self.calibration_level.next_cost(),
            ),
            RepairTrack::Containment => (
                self.containment_level.level(),
                self.containment_level.next_cost(),
            ),
        };
        let Some(cost) = cost else {
            return Err(RepairError::MaximumLevelReached);
        };
        let available = self.credit_wallet.available();
        if available < cost {
            return Err(RepairError::InsufficientCredits {
                required: cost,
                available,
            });
        }
        let level_after = level_before + 1;
        self.credit_wallet.spend(cost);
        match track {
            RepairTrack::Calibration => self.calibration_level = self.calibration_level.advance(),
            RepairTrack::Containment => self.containment_level = self.containment_level.advance(),
        }
        let purchase = RepairPurchase {
            id: RepairPurchaseId(self.next_repair_purchase_id),
            track,
            level_before,
            level_after,
            credits_spent: cost,
            credits_remaining: self.credit_wallet.available(),
            action_number: self.run.actions_used,
            tick: self.tick,
        };
        self.next_repair_purchase_id = self.next_repair_purchase_id.saturating_add(1);
        self.repair_purchases.push(purchase);
        Ok(purchase)
    }

    pub fn publication_limit(&self) -> u32 {
        PUBLICATION_LIMIT
    }

    pub fn publication_for(&self, id: HypothesisId) -> Option<&Publication> {
        self.publications
            .iter()
            .find(|publication| publication.hypothesis_id == id)
    }

    pub fn add_hypothesis(
        &mut self,
        cause: ObservableVariable,
        direction: HypothesisDirection,
        effect: ObservableVariable,
    ) -> Result<HypothesisId, NotebookError> {
        self.ensure_notebook_editable()?;
        self.ensure_hypothesis_unpublished_for_edit(None, cause, direction, effect)?;
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
        self.ensure_hypothesis_unpublished_for_edit(Some(id), cause, direction, effect)?;
        self.notebook.edit(id, cause, direction, effect)
    }

    pub fn remove_hypothesis(&mut self, id: HypothesisId) -> Result<(), NotebookError> {
        self.ensure_notebook_editable()?;
        if self.publication_for(id).is_some() {
            return Err(NotebookError::HypothesisAlreadyPublished);
        }
        self.notebook.remove(id)
    }

    pub fn publish_hypothesis(
        &mut self,
        id: HypothesisId,
    ) -> Result<Publication, PublicationError> {
        if self.lifecycle_enabled && self.run.status != RunStatus::Active {
            return Err(PublicationError::RunResolved);
        }
        if self.publications.len() as u32 >= PUBLICATION_LIMIT {
            return Err(PublicationError::PublicationLimitReached);
        }
        if self.publication_for(id).is_some() {
            return Err(PublicationError::HypothesisAlreadyPublished);
        }
        let hypothesis = self
            .notebook
            .hypotheses()
            .iter()
            .find(|hypothesis| hypothesis.id == id)
            .copied()
            .ok_or(PublicationError::HypothesisNotFound)?;
        let (evidence_strength, evidence_summary) =
            evaluate(hypothesis, &self.recipe, &self.runlog.events);
        let publication = Publication {
            id: self.next_publication_id,
            hypothesis_id: id,
            hypothesis,
            credits_awarded: evidence_strength.credits(),
            evidence_strength,
            evidence_summary,
            action_number: self.run.actions_used.saturating_add(1),
            tick: self.tick,
        };
        self.next_publication_id = self.next_publication_id.saturating_add(1);
        self.credit_wallet.award(publication.credits_awarded);
        self.publications.push(publication.clone());
        if self.lifecycle_enabled {
            self.run.actions_used = self.run.actions_used.saturating_add(1);
            self.resolve_after_action(false);
        }
        Ok(publication)
    }

    fn ensure_notebook_editable(&self) -> Result<(), NotebookError> {
        if self.lifecycle_enabled && self.run.status != RunStatus::Active {
            Err(NotebookError::RunResolved)
        } else {
            Ok(())
        }
    }

    fn ensure_hypothesis_unpublished_for_edit(
        &self,
        id: Option<HypothesisId>,
        _cause: ObservableVariable,
        _direction: HypothesisDirection,
        _effect: ObservableVariable,
    ) -> Result<(), NotebookError> {
        if let Some(id) = id {
            if self.publication_for(id).is_some() {
                return Err(NotebookError::HypothesisAlreadyPublished);
            }
        }
        Ok(())
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
        let base_contamination_cost = action.contamination_cost();
        let containment_reduction = self.containment_reduction();
        let effective_contamination_cost =
            base_contamination_cost.saturating_sub(containment_reduction);
        let mut measurements = Vec::new();
        self.apply_intervention(&action, &mut measurements)?;
        if action.ticks_time() {
            self.tick_once()?;
        }

        self.add_contamination(effective_contamination_cost);
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
            base_contamination_cost,
            containment_reduction,
            effective_contamination_cost,
        };
        self.runlog.push(event.clone());
        if self.lifecycle_enabled {
            self.run.actions_used += 1;
            self.update_objective_progress();
            self.resolve_after_action(true);
        }
        Ok(event)
    }

    fn resolve_after_action(&mut self, objective_evaluated: bool) {
        if objective_evaluated && self.run.objective_progress.is_complete() {
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
            self.publications.clone(),
            self.credit_wallet.earned(),
            self.credit_wallet.spent(),
            self.credit_wallet.available(),
            self.calibration_level,
            self.containment_level,
            self.repair_purchases.clone(),
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
        let calibration_multiplier = self.calibration_multiplier();
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
                measurements.extend(scan_population_with_calibration(
                    &self.state,
                    &mut self.rng,
                    self.tick,
                    self.contamination,
                    calibration_multiplier,
                ));
            }
            Intervention::ScanChemicals => {
                measurements.extend(scan_chemicals_with_calibration(
                    &self.state,
                    &mut self.rng,
                    self.tick,
                    self.contamination,
                    calibration_multiplier,
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
