use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::engine::interventions::Intervention;
use crate::engine::notebook::{Hypothesis, HypothesisDirection, HypothesisId, ObservableVariable};
use crate::engine::runlog::RunEvent;
use crate::engine::world::{WorldRecipe, WorldState};

pub const PUBLICATION_LIMIT: u32 = 4;
pub const MAX_RESEARCH_CREDITS: u32 = PUBLICATION_LIMIT * 3;
pub const MIN_EFFECT_DELTA: f32 = 0.5;
const MAX_PATH_DEPTH: u32 = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceStrength {
    Unsupported,
    Weak,
    Moderate,
    Strong,
}

impl EvidenceStrength {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unsupported => "UNSUPPORTED",
            Self::Weak => "WEAK",
            Self::Moderate => "MODERATE",
            Self::Strong => "STRONG",
        }
    }

    pub fn credits(self) -> u32 {
        match self {
            Self::Unsupported => 0,
            Self::Weak => 1,
            Self::Moderate => 2,
            Self::Strong => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PublicationRationale {
    NoRelevantEvidence,
    NoSupportedManipulation,
    EvidenceContradictsClaim,
    EvidenceTooLimited,
    EvidencePartiallySupportsClaim,
    EvidenceConsistentlySupportsClaim,
}

impl PublicationRationale {
    pub fn text(self) -> &'static str {
        match self {
            Self::NoRelevantEvidence => "No relevant intervention evidence was observed.",
            Self::NoSupportedManipulation => {
                "No supported direct manipulation exists for this cause."
            }
            Self::EvidenceContradictsClaim => {
                "The collected evidence does not justify this directional claim."
            }
            Self::EvidenceTooLimited => {
                "The collected evidence is too limited to support this claim."
            }
            Self::EvidencePartiallySupportsClaim => {
                "The collected evidence provides limited directional support."
            }
            Self::EvidenceConsistentlySupportsClaim => {
                "Repeated, consistent observations support this claim."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSummary {
    pub relevant_trials: u32,
    pub supporting_trials: u32,
    pub contradicting_trials: u32,
    pub inconclusive_trials: u32,
    pub distinct_intervention_directions: u32,
    pub rationale: PublicationRationale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Publication {
    pub id: u32,
    pub hypothesis_id: HypothesisId,
    pub hypothesis: Hypothesis,
    pub evidence_strength: EvidenceStrength,
    pub credits_awarded: u32,
    pub evidence_summary: EvidenceSummary,
    pub action_number: u32,
    pub tick: u32,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PublicationError {
    #[error("run has already resolved")]
    RunResolved,
    #[error("hypothesis was not found")]
    HypothesisNotFound,
    #[error("hypothesis has already been published")]
    HypothesisAlreadyPublished,
    #[error("publication limit reached")]
    PublicationLimitReached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralRelation {
    Positive,
    Negative,
    Mixed,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrialResult {
    Supporting,
    Contradicting,
    Inconclusive,
}

#[derive(Debug, Clone, Copy)]
struct Trial {
    cause_delta: f32,
    effect_delta: f32,
}

pub fn evaluate(
    hypothesis: Hypothesis,
    recipe: &WorldRecipe,
    events: &[RunEvent],
) -> (EvidenceStrength, EvidenceSummary) {
    let Some(manipulations) = direct_manipulations(hypothesis.cause) else {
        return unsupported(PublicationRationale::NoSupportedManipulation);
    };
    let trials = collect_trials(hypothesis, manipulations, recipe.initial_state, events);
    if trials.is_empty() {
        return unsupported(PublicationRationale::NoRelevantEvidence);
    }

    let mut supporting = 0_u32;
    let mut contradicting = 0_u32;
    let mut inconclusive = 0_u32;
    let mut directions = Vec::new();
    for trial in trials {
        if trial.cause_delta > 0.0 {
            directions.push(1_i8);
        } else if trial.cause_delta < 0.0 {
            directions.push(-1_i8);
        }
        match classify_trial(hypothesis.direction, trial) {
            TrialResult::Supporting => supporting += 1,
            TrialResult::Contradicting => contradicting += 1,
            TrialResult::Inconclusive => inconclusive += 1,
        }
    }
    directions.sort_unstable();
    directions.dedup();
    let distinct_directions = directions.len() as u32;
    let relevant = supporting + contradicting + inconclusive;
    let rationale;
    let strength;

    if !structurally_compatible(recipe, hypothesis) {
        strength = EvidenceStrength::Unsupported;
        rationale = PublicationRationale::EvidenceContradictsClaim;
    } else if supporting == 0 || contradicting >= supporting {
        strength = EvidenceStrength::Unsupported;
        rationale = if contradicting > 0 {
            PublicationRationale::EvidenceContradictsClaim
        } else {
            PublicationRationale::EvidenceTooLimited
        };
    } else if supporting >= 3
        && contradicting == 0
        && (distinct_directions >= 2 || !manipulations.supports_both_directions)
        && inconclusive == 0
    {
        strength = EvidenceStrength::Strong;
        rationale = PublicationRationale::EvidenceConsistentlySupportsClaim;
    } else if supporting >= 2 && supporting > contradicting && contradicting <= 1 {
        strength = EvidenceStrength::Moderate;
        rationale = PublicationRationale::EvidencePartiallySupportsClaim;
    } else {
        strength = EvidenceStrength::Weak;
        rationale = PublicationRationale::EvidencePartiallySupportsClaim;
    }

    (
        strength,
        EvidenceSummary {
            relevant_trials: relevant,
            supporting_trials: supporting,
            contradicting_trials: contradicting,
            inconclusive_trials: inconclusive,
            distinct_intervention_directions: distinct_directions,
            rationale,
        },
    )
}

#[derive(Clone, Copy)]
struct DirectManipulations {
    interventions: &'static [InterventionKind],
    supports_both_directions: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterventionKind {
    SetUvLow,
    SetUvHigh,
    AddNutrient,
    AddToxin,
    NeutralizeToxin,
    RemoveFungus,
    SterilizeFungus,
    RemoveBacteria,
}

const UV_MANIPULATIONS: [InterventionKind; 2] =
    [InterventionKind::SetUvLow, InterventionKind::SetUvHigh];
const NUTRIENT_MANIPULATIONS: [InterventionKind; 1] = [InterventionKind::AddNutrient];
const TOXIN_MANIPULATIONS: [InterventionKind; 2] = [
    InterventionKind::AddToxin,
    InterventionKind::NeutralizeToxin,
];
const FUNGUS_MANIPULATIONS: [InterventionKind; 2] = [
    InterventionKind::RemoveFungus,
    InterventionKind::SterilizeFungus,
];
const BACTERIA_MANIPULATIONS: [InterventionKind; 1] = [InterventionKind::RemoveBacteria];

fn direct_manipulations(cause: ObservableVariable) -> Option<DirectManipulations> {
    match cause {
        ObservableVariable::UvLevel => Some(DirectManipulations {
            interventions: &UV_MANIPULATIONS,
            supports_both_directions: true,
        }),
        ObservableVariable::NutrientConcentration => Some(DirectManipulations {
            interventions: &NUTRIENT_MANIPULATIONS,
            supports_both_directions: false,
        }),
        ObservableVariable::ToxinConcentration => Some(DirectManipulations {
            interventions: &TOXIN_MANIPULATIONS,
            supports_both_directions: true,
        }),
        ObservableVariable::FungusPopulation => Some(DirectManipulations {
            interventions: &FUNGUS_MANIPULATIONS,
            supports_both_directions: false,
        }),
        ObservableVariable::BacteriaPopulation => Some(DirectManipulations {
            interventions: &BACTERIA_MANIPULATIONS,
            supports_both_directions: false,
        }),
        ObservableVariable::PlantPopulation => None,
    }
}

fn collect_trials(
    hypothesis: Hypothesis,
    manipulations: DirectManipulations,
    initial_state: WorldState,
    events: &[RunEvent],
) -> Vec<Trial> {
    let mut trials = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !matches_kind(&event.intervention, manipulations.interventions) {
            continue;
        }
        let before = if index == 0 {
            initial_state
        } else {
            events[index - 1].state_snapshot
        };
        let cause_delta =
            hypothesis.cause.value(&event.state_snapshot) - hypothesis.cause.value(&before);
        if cause_delta.abs() < f32::EPSILON {
            continue;
        }

        let observed = if hypothesis.effect == ObservableVariable::UvLevel {
            true
        } else {
            let required_scan = required_scan(hypothesis.effect);
            let mut observed = false;
            for later in events.iter().skip(index + 1) {
                if matches_kind(&later.intervention, manipulations.interventions) {
                    break;
                }
                if scan_matches(&later.intervention, required_scan) {
                    observed = true;
                    break;
                }
            }
            observed
        };
        if observed {
            let effect_delta =
                hypothesis.effect.value(&event.state_snapshot) - hypothesis.effect.value(&before);
            trials.push(Trial {
                cause_delta,
                effect_delta,
            });
        }
    }
    trials
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequiredScan {
    Population,
    Chemicals,
}

fn required_scan(effect: ObservableVariable) -> RequiredScan {
    match effect {
        ObservableVariable::PlantPopulation
        | ObservableVariable::FungusPopulation
        | ObservableVariable::BacteriaPopulation => RequiredScan::Population,
        ObservableVariable::ToxinConcentration | ObservableVariable::NutrientConcentration => {
            RequiredScan::Chemicals
        }
        ObservableVariable::UvLevel => unreachable!("UV is observed directly"),
    }
}

fn scan_matches(intervention: &Intervention, required: RequiredScan) -> bool {
    matches!(
        (required, intervention),
        (RequiredScan::Population, Intervention::ScanPopulation)
            | (RequiredScan::Chemicals, Intervention::ScanChemicals)
    )
}

fn matches_kind(intervention: &Intervention, kinds: &[InterventionKind]) -> bool {
    kinds.iter().any(|kind| {
        matches!(
            (kind, intervention),
            (InterventionKind::SetUvLow, Intervention::SetUvLow)
                | (InterventionKind::SetUvHigh, Intervention::SetUvHigh)
                | (InterventionKind::AddNutrient, Intervention::AddNutrient(_))
                | (InterventionKind::AddToxin, Intervention::AddToxin(_))
                | (
                    InterventionKind::NeutralizeToxin,
                    Intervention::NeutralizeToxin(_)
                )
                | (InterventionKind::RemoveFungus, Intervention::RemoveFungus)
                | (
                    InterventionKind::SterilizeFungus,
                    Intervention::SterilizeSample
                )
                | (
                    InterventionKind::RemoveBacteria,
                    Intervention::RemoveBacteria
                )
        )
    })
}

fn classify_trial(direction: HypothesisDirection, trial: Trial) -> TrialResult {
    if trial.effect_delta.abs() < MIN_EFFECT_DELTA {
        return TrialResult::Inconclusive;
    }
    let expected_sign = match (trial.cause_delta.is_sign_positive(), direction) {
        (true, HypothesisDirection::Increases) | (false, HypothesisDirection::Decreases) => 1.0,
        _ => -1.0,
    };
    if trial.effect_delta.signum() == expected_sign {
        TrialResult::Supporting
    } else {
        TrialResult::Contradicting
    }
}

fn unsupported(rationale: PublicationRationale) -> (EvidenceStrength, EvidenceSummary) {
    (
        EvidenceStrength::Unsupported,
        EvidenceSummary {
            relevant_trials: 0,
            supporting_trials: 0,
            contradicting_trials: 0,
            inconclusive_trials: 0,
            distinct_intervention_directions: 0,
            rationale,
        },
    )
}

fn structurally_compatible(recipe: &WorldRecipe, hypothesis: Hypothesis) -> bool {
    let relation = structural_relation(recipe, hypothesis.cause.node(), hypothesis.effect.node());
    matches!(
        (relation, hypothesis.direction),
        (StructuralRelation::Positive, HypothesisDirection::Increases)
            | (StructuralRelation::Negative, HypothesisDirection::Decreases)
    )
}

fn structural_relation(
    recipe: &WorldRecipe,
    cause: crate::engine::ids::NodeId,
    effect: crate::engine::ids::NodeId,
) -> StructuralRelation {
    let mut path = vec![cause];
    let mut signs = PathSigns {
        positive: false,
        negative: false,
    };
    walk_paths(recipe, cause, effect, 0, 1_i8, &mut path, &mut signs);
    match (signs.positive, signs.negative) {
        (true, false) => StructuralRelation::Positive,
        (false, true) => StructuralRelation::Negative,
        (true, true) => StructuralRelation::Mixed,
        (false, false) => StructuralRelation::Absent,
    }
}

struct PathSigns {
    positive: bool,
    negative: bool,
}

fn walk_paths(
    recipe: &WorldRecipe,
    current: crate::engine::ids::NodeId,
    target: crate::engine::ids::NodeId,
    depth: u32,
    sign: i8,
    path: &mut Vec<crate::engine::ids::NodeId>,
    signs: &mut PathSigns,
) {
    if depth >= MAX_PATH_DEPTH {
        return;
    }
    let mut edges: Vec<_> = recipe
        .edges
        .iter()
        .filter(|edge| edge.from == current)
        .collect();
    edges.sort_by_key(|edge| (edge.to.as_index(), edge.weight.to_bits()));
    for edge in edges {
        let next_sign = if edge.weight >= 0.0 { sign } else { -sign };
        if edge.to == target {
            if next_sign > 0 {
                signs.positive = true;
            } else {
                signs.negative = true;
            }
        } else if !path.contains(&edge.to) {
            path.push(edge.to);
            walk_paths(recipe, edge.to, target, depth + 1, next_sign, path, signs);
            path.pop();
        }
    }
}
