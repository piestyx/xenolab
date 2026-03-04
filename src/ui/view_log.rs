use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let events = app.simulator.events();
    let viewport_height = area.height.saturating_sub(2) as usize;
    let max_start = events.len().saturating_sub(viewport_height);
    let start = app.log_scroll.min(max_start);
    let end = (start + viewport_height).min(events.len());

    let lines: Vec<String> = events[start..end]
        .iter()
        .map(|event| {
            format!(
                "tick={:<3} action={:<20} meas={} contam={:.1}",
                event.tick_index,
                event.intervention.label(),
                event.measurements.len(),
                event.contamination
            )
        })
        .collect();

    let body = if lines.is_empty() {
        String::from("Runlog is empty.")
    } else {
        lines.join("\n")
    };

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title("Run Log (up/down to scroll)")
            .borders(Borders::ALL),
    );
    frame.render_widget(paragraph, area);
}
