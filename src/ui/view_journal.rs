use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::app::App;

pub fn render_journal(f: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "XENOLAB FIELD JOURNAL\n\nSeed: {}\nTick: {}\nObjective: {}\nProgress: {}\nActions remaining: {}\n\nSituation\nA contained xenobiology sample is active in the wet lab.\nYour interventions shift populations and chemistry every time the clock advances.\nMeasurements are noisy and only scans report instrument readings.\n\nObjective\n{}\n{}\n\nContamination\n- Invasive actions accumulate contamination; scans remain contamination-free.\n- 0-19 is Stable, 20-29 Compromised, 30-39 Critical, and 40 loses containment.\n- Compromised scans use 1.5x noise; Critical scans use 2.25x noise.\n- Contamination does not alter the ecosystem or objective evaluation.\n- Objective completion wins if it occurs on the same action as containment loss.\n\nNotebook and publication\n- The Notebook records your theory; publishing asks the engine to evaluate it against this run's evidence.\n- Publish only after a direct intervention and the required follow-up observation.\n- Publication costs one action, is permanent, and locks that hypothesis from editing or removal.\n- Unsupported claims earn no credits; repeated consistent evidence can earn stronger results.\n- Credits are run-local, capped at 12, and cannot yet be spent.\n\nRules\n- Every accepted intervention, scan, or publication consumes one of 30 actions.\n- Most interventions advance exactly one simulation tick.\n- Scan and publication actions do not advance time.\n- Objective progress uses true state after each accepted simulation action.\n- A completed objective wins; 30 actions without success fails and locks the run.\n\nControls\n- q: quit\n- 1/2/3/4: switch tabs\n- Tab 1 (Lab): arrows choose action, Enter applies\n- Tab 2 (Journal): Up/Down or j/k scroll, PageUp/PageDown jump\n- Tab 3 (Log): Up/Down scroll events and publication section\n- Tab 4 (Notebook): a add, e edit, d delete, p publish, arrows navigate\n- Resolved run: r same seed, n new seed, q quit\n",
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
