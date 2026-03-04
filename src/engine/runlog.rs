use serde::{Deserialize, Serialize};

use crate::engine::interventions::Intervention;
use crate::engine::measurement::MeasurementRecord;
use crate::engine::world::WorldState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEvent {
    pub tick_index: u32,
    pub intervention: Intervention,
    pub measurements: Vec<MeasurementRecord>,
    pub state_snapshot: WorldState,
    pub contamination: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RunLog {
    pub events: Vec<RunEvent>,
}

impl RunLog {
    pub fn push(&mut self, event: RunEvent) {
        self.events.push(event);
    }
}

pub fn hash_events(events: &[RunEvent]) -> blake3::Hash {
    match serde_json::to_vec(events) {
        Ok(bytes) => blake3::hash(&bytes),
        Err(_) => blake3::hash(&[]),
    }
}
