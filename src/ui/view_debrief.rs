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
        RunStatus::Active => "RUN ACTIVE",
    };
    let failure = match debrief.failure_reason {
        Some(RunFailure::ActionBudgetExhausted) => "Action budget exhausted",
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
        "{outcome}\n\nSeed: {}\nObjective: {}\nOutcome: {:?}\nFailure: {failure}\n\nActions: {} / {}\nFinal tick: {}\nContamination: {:.2}\n\nPlant: {:.2}\nFungus: {:.2}\nBacteria: {:.2}\nToxin: {:.2}\nNutrient: {:.2}\n\nObjective progress: {} / {}\nRun-event hash: {}\n\nControls\nr same seed | n new seed | q quit{prompt}",
        debrief.seed,
        debrief.objective.label(),
        debrief.outcome,
        debrief.actions_used,
        debrief.action_limit,
        debrief.final_tick,
        debrief.final_contamination,
        debrief.final_plant,
        debrief.final_fungus,
        debrief.final_bacteria,
        debrief.final_toxin,
        debrief.final_nutrient,
        debrief.objective_progress.current,
        debrief.objective_progress.required,
        debrief.event_hash,
    );
    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Debrief").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
