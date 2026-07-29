use serde::{Deserialize, Serialize};

pub const COMPROMISED_THRESHOLD: f32 = 20.0;
pub const CRITICAL_THRESHOLD: f32 = 30.0;
pub const CONTAINMENT_LOST_THRESHOLD: f32 = 40.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContaminationLevel {
    Stable,
    Compromised,
    Critical,
    Lost,
}

impl ContaminationLevel {
    pub fn from_value(value: f32) -> Self {
        if value >= CONTAINMENT_LOST_THRESHOLD {
            Self::Lost
        } else if value >= CRITICAL_THRESHOLD {
            Self::Critical
        } else if value >= COMPROMISED_THRESHOLD {
            Self::Compromised
        } else {
            Self::Stable
        }
    }

    pub fn noise_multiplier(self) -> f32 {
        match self {
            Self::Stable => 1.0,
            Self::Compromised => 1.5,
            Self::Critical | Self::Lost => 2.25,
        }
    }

    pub fn next_threshold(self) -> Option<u32> {
        match self {
            Self::Stable => Some(COMPROMISED_THRESHOLD as u32),
            Self::Compromised => Some(CRITICAL_THRESHOLD as u32),
            Self::Critical => Some(CONTAINMENT_LOST_THRESHOLD as u32),
            Self::Lost => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Compromised => "COMPROMISED",
            Self::Critical => "CRITICAL",
            Self::Lost => "CONTAINMENT LOST",
        }
    }
}
