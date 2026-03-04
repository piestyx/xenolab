use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

use crate::engine::graph::Graph;
use crate::engine::ids::NodeId;
use crate::engine::interventions::Intervention;
use crate::engine::math;
use crate::engine::measurement::{scan_chemicals, scan_population, sample_standard_normal};
use crate::engine::runlog::{RunEvent, RunLog};
use crate::engine::world::{WorldRecipe, WorldState};

const INFLUENCE_SCALE: f32 = crate::worldgen::spec::INFLUENCE_SCALE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseMode {
    Normal,
    Disabled,
}

#[derive(Debug, Error)]
pub enum SimError {
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
    noise_mode: NoiseMode,
    rng: ChaCha8Rng,
    runlog: RunLog,
}

impl Simulator {
    pub fn new(recipe: WorldRecipe) -> Self {
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
            noise_mode: NoiseMode::Normal,
            rng: ChaCha8Rng::seed_from_u64(rng_seed),
            runlog: RunLog::default(),
        }
    }

    pub fn new_no_noise(recipe: WorldRecipe) -> Self {
        let mut sim = Self::new(recipe);
        sim.noise_mode = NoiseMode::Disabled;
        sim
    }

    pub fn with_noise_mode(recipe: WorldRecipe, noise_mode: NoiseMode) -> Self {
        let mut sim = Self::new(recipe);
        sim.noise_mode = noise_mode;
        sim
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

    pub fn events(&self) -> &[RunEvent] {
        &self.runlog.events
    }

    pub fn apply(
        &mut self,
        action: crate::engine::interventions::Intervention,
    ) -> Result<crate::engine::runlog::RunEvent, crate::engine::sim::SimError> {
        let mut measurements = Vec::new();
        self.apply_intervention(&action, &mut measurements)?;
        if action.ticks_time() {
            self.tick_once()?;
        }

        let event = RunEvent {
            tick_index: self.tick,
            intervention: action,
            measurements,
            state_snapshot: self.state,
            contamination: self.contamination,
        };
        self.runlog.push(event.clone());
        Ok(event)
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
                self.contamination = math::clamp01(self.contamination + 15.0);
            }
            Intervention::ScanPopulation => {
                measurements.extend(scan_population(&self.state, &mut self.rng, self.tick));
            }
            Intervention::ScanChemicals => {
                measurements.extend(scan_chemicals(&self.state, &mut self.rng, self.tick));
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
            let updated = math::apply_update(current, bias, influence, INFLUENCE_SCALE, noise);
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
            return Err(SimError::InvalidEdgeIndex { from: to_idx, to: to_idx });
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
}

fn validate_non_negative(delta: f32, label: &'static str) -> Result<(), SimError> {
    if delta.is_finite() && delta >= 0.0 {
        Ok(())
    } else {
        Err(SimError::InvalidDelta(label))
    }
}
