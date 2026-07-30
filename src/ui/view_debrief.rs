use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::engine::run::{RunFailure, RunStatus};
use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let Some(debrief) = app.simulator.debrief() else {
        return;
    };

    let outcome = match debrief.outcome {
        RunStatus::Won => "RUN COMPLETE",
        RunStatus::Failed(RunFailure::ActionBudgetExhausted) => "RUN FAILED",
        RunStatus::Failed(RunFailure::ContainmentLost) => "RUN FAILED",
        RunStatus::Active => "RUN ACTIVE",
    };
    let failure = match debrief.failure_reason {
        Some(RunFailure::ActionBudgetExhausted) => "Action budget exhausted",
        Some(RunFailure::ContainmentLost) => "Containment lost",
        None => "-",
    };
    let prompt = match &app.pending_seed_input {
        Some(input) => format!(
            "\nNew seed (decimal u64): {input}_\nEnter confirm | Esc cancel\n{}",
            app.status_message
        ),
        None => String::new(),
    };
    let text = format!(
        "{outcome}\n\nSeed: {}\nObjective: {}\nOutcome: {:?}\nFailure: {failure}\n\nActions: {} / {}\nFinal tick: {}\nContamination: {:.2} ({})\nPeak contamination: {:.2}\nCompromised scans: {}\nCritical scans: {}\n\nPlant: {:.2}\nFungus: {:.2}\nBacteria: {:.2}\nToxin: {:.2}\nNutrient: {:.2}\n\nObjective progress: {} / {}\nRun-event hash: {}\n\nCredits: {} earned / {} spent / {} available (max {})\nRepairs: Calibration L{} (scan ×{:.2}) | Containment L{} (cost -{})\nPublications: {} / {}\n{}\n\nRecorded hypotheses: {}\n{}\n\nRepairs purchased: {}\n{}\n\nControls\nr same seed | n new seed | 4 notebook | 5 repairs | q quit{prompt}",
        debrief.seed,
        debrief.objective.label(),
        debrief.outcome,
        debrief.actions_used,
        debrief.action_limit,
        debrief.final_tick,
        debrief.final_contamination,
        debrief.final_contamination_level.label(),
        debrief.peak_contamination,
        debrief.compromised_scans,
        debrief.critical_scans,
        debrief.final_plant,
        debrief.final_fungus,
        debrief.final_bacteria,
        debrief.final_toxin,
        debrief.final_nutrient,
        debrief.objective_progress.current,
        debrief.objective_progress.required,
        debrief.event_hash,
        debrief.credits_earned,
        debrief.credits_spent,
        debrief.credits_remaining,
        debrief.publication_limit * 3,
        debrief.calibration_level.level(),
        debrief.calibration_multiplier,
        debrief.containment_level.level(),
        debrief.containment_reduction,
        debrief.publications_used,
        debrief.publication_limit,
        debrief
            .publications
            .iter()
            .map(|publication| {
                format!(
                    "Publication [{}] {} +{} credits: {}\n  trials {}/{}/{} /{} — {}",
                    publication.id,
                    publication.evidence_strength.label(),
                    publication.credits_awarded,
                    publication.hypothesis.sentence(),
                    publication.evidence_summary.relevant_trials,
                    publication.evidence_summary.supporting_trials,
                    publication.evidence_summary.contradicting_trials,
                    publication.evidence_summary.inconclusive_trials,
                    publication.evidence_summary.rationale.text()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        debrief.repair_purchases.len(),
        debrief
            .repair_purchases
            .iter()
            .map(|purchase| format!(
                "[{}] {} L{}→L{} spent {} remaining {} (action {}, tick {})",
                purchase.id.0,
                purchase.track.label(),
                purchase.level_before,
                purchase.level_after,
                purchase.credits_spent,
                purchase.credits_remaining,
                purchase.action_number,
                purchase.tick
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        debrief.notebook.len(),
        debrief
            .notebook
            .iter()
            .map(|hypothesis| format!("[{}] {}", hypothesis.id.0, hypothesis.sentence()))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Debrief").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
