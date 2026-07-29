use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::app::App;

pub fn render_journal(f: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "XENOLAB FIELD JOURNAL\n\nSeed: {}\nTick: {}\nObjective: {}\nProgress: {}\nActions remaining: {}\n\nSituation\nA contained xenobiology sample is active in the wet lab.\nYour interventions shift populations and chemistry every time the clock advances.\nMeasurements are noisy and only scans report instrument readings.\n\nObjective\n{}\n{}\n\nRules\n- Every accepted intervention or scan consumes one of 30 actions.\n- Most interventions advance exactly one simulation tick.\n- Scan interventions consume an action but do not advance time.\n- Objective progress uses true state after each accepted action.\n- A completed objective wins; 30 actions without success fails and locks the run.\n\nControls\n- q: quit\n- 1/2/3: switch tabs\n- Tab 1 (Lab): arrows choose action, Enter applies\n- Tab 2 (Journal): Up/Down or j/k scroll, PageUp/PageDown jump\n- Tab 3 (Log): Up/Down scroll events\n- Resolved run: r same seed, n new seed, q quit\n",
        app.seed,
        app.simulator.tick_index(),
        app.objective.label(),
        app.objective_progress_text(),
        app.actions_remaining(),
        app.objective_goal_text(),
        app.objective_failure_text()
    );

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Journal").borders(Borders::ALL))
        .scroll((app.journal_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}
