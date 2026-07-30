use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::app::App;

pub fn render_journal(f: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "XENOLAB FIELD JOURNAL\n\nSeed: {}\nTick: {}\nObjective: {}\nProgress: {}\nActions remaining: {}\n\nSituation\nA contained xenobiology sample is active in the wet lab.\nInterventions shift populations and chemistry when time advances.\nMeasurements are noisy; scans report instrument readings.\n\nContamination\n- Invasive actions accumulate contamination; scans remain contamination-free.\n- 0-19 Stable, 20-29 Compromised, 30-39 Critical, and 40 loses containment.\n- Compromised scans use 1.5x noise; Critical scans use 2.25x noise.\n- Objective completion wins if it occurs on the same action as containment loss.\n\nNotebook and publication\n- The Notebook records theory using observable variables.\n- Publication costs one action, is permanent, and evaluates current-run evidence.\n- Unsupported claims earn no credits; repeated consistent evidence can earn stronger results.\n\nRepairs\n- Publication credits are run-local and can be spent in the Repairs tab.\n- Calibration reduces future scan noise; Containment reduces future contamination costs.\n- Each track has two levels. Repairs consume no actions, ticks, contamination, or RNG.\n- Repairs never rewrite previous events and reset with the run.\n\nRules\n- Every accepted intervention, scan, or publication consumes one of 30 actions.\n- Most interventions advance one simulation tick; scans and publications do not.\n- A completed objective wins; 30 actions without success fails and locks the run.\n\nControls\n- q: quit\n- 1/2/3/4/5: switch tabs\n- Tab 1 (Lab): arrows choose action, Enter applies\n- Tab 2 (Journal): Up/Down or j/k scroll\n- Tab 3 (Log): Up/Down scroll events and separate records\n- Tab 4 (Notebook): a add, e edit, d delete, p publish\n- Tab 5 (Repairs): arrows select, Enter purchase after confirmation\n- Resolved run: r same seed, n new seed, q quit\n",
        app.seed,
        app.simulator.tick_index(),
        app.objective.label(),
        app.objective_progress_text(),
        app.actions_remaining(),
    );

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Journal").borders(Borders::ALL))
        .scroll((app.journal_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}
