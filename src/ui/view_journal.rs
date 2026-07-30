use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::app::App;

pub fn render_journal(f: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "XENOLAB FIELD JOURNAL\n\nRun\nSeed: {} | Tick: {} | Actions remaining: {}\nObjective: {}\n{}\n\nBriefing\n{}\n\nHow to play\nEvery accepted intervention, scan, or publication uses 1 of 30 actions.\nInterventions and Advance Time move the simulation one tick; scans do not.\nA scan still costs an action. Objective progress is checked after accepted actions;\nqualifying evaluations are consecutive, and any failed evaluation resets the hold.\n\nRisk\nContamination: 0-19 Stable, 20-29 Compromised, 30-39 Critical, 40 loses containment.\nCompromised and Critical scans are noisier. Objective win takes precedence on the same action.\n\nResearch loop\nNotebook: record X increases/decreases Y using the six observable variables.\nPublication: select a hypothesis and publish after a direct cause intervention plus the\nrequired follow-up population or chemical scan. Publication costs 1 action and is permanent.\nUnsupported claims earn 0; repeated, consistent evidence earns stronger results.\n\nCredits and Repairs\nPublications award run-local credits. Repairs spends them without actions, ticks,\ncontamination, or RNG: Calibration improves future scan precision; Containment lowers\nfuture contamination costs. Both tracks reset when restarting.\n\nControls\n1-5 tabs | ? help | q quit | Lab: arrows/Enter apply, x repeat last action\nJournal: arrows, j/k, PageUp/PageDown scroll | Log: arrows, a all, i interventions,\nm measurements, p publications, r repairs | Notebook: a/e/d/p, Enter, Esc\nRepairs: arrows/Enter, Esc | resolved: r same seed, n new seed, q quit\n",
        app.seed,
        app.simulator.tick_index(),
        app.actions_remaining(),
        app.objective.label(),
        app.objective_progress_text(),
        match app.objective {
            crate::engine::ids::ObjectiveId::StabilizePlant =>
                "Stabilize Plant: plant population must remain at 60+ for 3 consecutive evaluations.",
            crate::engine::ids::ObjectiveId::Detox =>
                "Detox: toxin concentration must remain at 15 or below for 3 consecutive evaluations.",
            crate::engine::ids::ObjectiveId::PreventCollapse =>
                "Prevent Collapse: plant and bacteria must both remain at 25+ for 3 consecutive evaluations.",
        },
    );

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Journal").borders(Borders::ALL))
        .scroll((app.journal_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}
