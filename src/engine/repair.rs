use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::engine::publication::MAX_RESEARCH_CREDITS;

pub const REPAIR_MAX_LEVEL: u8 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepairTrack {
    Calibration,
    Containment,
}

impl RepairTrack {
    pub fn label(self) -> &'static str {
        match self {
            Self::Calibration => "Calibration",
            Self::Containment => "Containment",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CalibrationLevel {
    Level0,
    Level1,
    Level2,
}

impl CalibrationLevel {
    pub fn level(self) -> u8 {
        match self {
            Self::Level0 => 0,
            Self::Level1 => 1,
            Self::Level2 => 2,
        }
    }

    pub fn noise_multiplier(self) -> f32 {
        match self {
            Self::Level0 => 1.0,
            Self::Level1 => 0.8,
            Self::Level2 => 0.6,
        }
    }

    pub fn next_cost(self) -> Option<u32> {
        match self {
            Self::Level0 => Some(2),
            Self::Level1 => Some(4),
            Self::Level2 => None,
        }
    }

    pub fn advance(self) -> Self {
        match self {
            Self::Level0 => Self::Level1,
            Self::Level1 | Self::Level2 => Self::Level2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContainmentLevel {
    Level0,
    Level1,
    Level2,
}

impl ContainmentLevel {
    pub fn level(self) -> u8 {
        match self {
            Self::Level0 => 0,
            Self::Level1 => 1,
            Self::Level2 => 2,
        }
    }

    pub fn contamination_reduction(self) -> u32 {
        match self {
            Self::Level0 => 0,
            Self::Level1 => 1,
            Self::Level2 => 2,
        }
    }

    pub fn next_cost(self) -> Option<u32> {
        match self {
            Self::Level0 => Some(2),
            Self::Level1 => Some(4),
            Self::Level2 => None,
        }
    }

    pub fn advance(self) -> Self {
        match self {
            Self::Level0 => Self::Level1,
            Self::Level1 | Self::Level2 => Self::Level2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreditWallet {
    earned: u32,
    spent: u32,
}

impl CreditWallet {
    pub fn new() -> Self {
        Self {
            earned: 0,
            spent: 0,
        }
    }

    pub fn earned(&self) -> u32 {
        self.earned
    }

    pub fn spent(&self) -> u32 {
        self.spent
    }

    pub fn available(&self) -> u32 {
        self.earned.saturating_sub(self.spent)
    }

    pub fn maximum_possible(&self) -> u32 {
        MAX_RESEARCH_CREDITS
    }

    pub(crate) fn award(&mut self, credits: u32) {
        self.earned = self
            .earned
            .saturating_add(credits)
            .min(MAX_RESEARCH_CREDITS);
    }

    pub(crate) fn spend(&mut self, credits: u32) {
        self.spent = self.spent.saturating_add(credits).min(self.earned);
    }
}

impl Default for CreditWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RepairError {
    #[error("insufficient credits: requires {required}, has {available}")]
    InsufficientCredits { required: u32, available: u32 },
    #[error("repair track is already at maximum level")]
    MaximumLevelReached,
    #[error("run has already resolved")]
    RunResolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepairPurchaseId(pub u32);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairPurchase {
    pub id: RepairPurchaseId,
    pub track: RepairTrack,
    pub level_before: u8,
    pub level_after: u8,
    pub credits_spent: u32,
    pub credits_remaining: u32,
    pub action_number: u32,
    pub tick: u32,
}
