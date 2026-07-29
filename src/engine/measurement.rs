use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::engine::ids::NodeId;
use crate::engine::world::{clamp_0_100, WorldState};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Instrument {
    BioScanner,
    Spectrometer,
}

impl Instrument {
    pub fn sigma(self) -> f32 {
        match self {
            Self::BioScanner => 0.05,
            Self::Spectrometer => 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeasurementRecord {
    pub tick_index: u32,
    pub instrument: Instrument,
    pub node: NodeId,
    pub true_value: f32,
    pub measured_value: f32,
}

pub fn scan_population(
    state: &WorldState,
    rng: &mut ChaCha8Rng,
    tick_index: u32,
) -> Vec<MeasurementRecord> {
    let nodes = [NodeId::PlantPop, NodeId::FungusLoad, NodeId::BacteriaPop];
    measure_nodes(state, rng, tick_index, Instrument::BioScanner, &nodes)
}

pub fn scan_chemicals(
    state: &WorldState,
    rng: &mut ChaCha8Rng,
    tick_index: u32,
) -> Vec<MeasurementRecord> {
    let nodes = [NodeId::Toxin, NodeId::Nutrient];
    measure_nodes(state, rng, tick_index, Instrument::Spectrometer, &nodes)
}

fn measure_nodes(
    state: &WorldState,
    rng: &mut ChaCha8Rng,
    tick_index: u32,
    instrument: Instrument,
    nodes: &[NodeId],
) -> Vec<MeasurementRecord> {
    let sigma = instrument.sigma();
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        let true_value = state.get(*node);
        let measured_value = measure_value(true_value, sigma, rng);
        out.push(MeasurementRecord {
            tick_index,
            instrument,
            node: *node,
            true_value,
            measured_value,
        });
    }
    out
}

fn measure_value(true_value: f32, sigma: f32, rng: &mut ChaCha8Rng) -> f32 {
    let z = sample_standard_normal(rng);
    let multiplier = 1.0 + z * sigma;
    clamp_0_100(true_value * multiplier)
}

pub fn sample_standard_normal(rng: &mut ChaCha8Rng) -> f32 {
    // Irwin-Hall approximation: sum(U[0,1]) - 6 approximates N(0,1).
    let mut sum = 0.0_f32;
    for _ in 0..12 {
        sum += rng.gen::<f32>();
    }
    sum - 6.0
}
