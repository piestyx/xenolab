use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::app::App;

pub fn render_journal(f: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        "XENOLAB FIELD JOURNAL\n\nSeed: {}\nTick: {}\nObjective: {}\n\nSituation\nA contained xenobiology sample is active in the wet lab.\nYour interventions shift populations and chemistry every time the clock advances.\nMeasurements are noisy and only scans report instrument readings.\n\nObjective\n{}\n{}\n\nRules\n- Most interventions advance exactly one simulation tick.\n- Scan interventions do not advance time and only report measurements.\n- Organisms, chemicals, and latent dynamics include deterministic seeded noise.\n- Contamination is tracked and can rise during sterilization procedures.\n\nControls\n- q: quit\n- 1/2/3: switch tabs\n- Tab 1 (Lab): arrows choose action, Enter applies\n- Tab 2 (Journal): Up/Down or j/k scroll, PageUp/PageDown jump\n- Tab 3 (Log): Up/Down scroll events\n",
        app.seed,
        app.simulator.tick_index(),
        app.objective.label(),
        app.objective_goal_text(),
        app.objective_failure_text()
    );

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Journal").borders(Borders::ALL))
        .scroll((app.journal_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}
