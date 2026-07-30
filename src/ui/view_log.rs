use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let event_lines: Vec<String> = app
        .simulator
        .events()
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
    let mut all_lines = vec!["Gameplay events".to_string()];
    if event_lines.is_empty() {
        all_lines.push("Runlog is empty.".to_string());
    } else {
        all_lines.extend(event_lines);
    }
    all_lines.push(String::new());
    all_lines.push("Publications (separate from RunEvent hash)".to_string());
    if app.simulator.publications().is_empty() {
        all_lines.push("No publications.".to_string());
    } else {
        for publication in app.simulator.publications() {
            all_lines.push(format!(
                "action={:<3} [{}] {} — +{} credits",
                publication.action_number,
                publication.evidence_strength.label(),
                publication.hypothesis.sentence(),
                publication.credits_awarded,
            ));
            all_lines.push(format!(
                "  {}",
                publication.evidence_summary.rationale.text()
            ));
        }
    }
    let viewport_height = area.height.saturating_sub(2) as usize;
    let max_start = all_lines.len().saturating_sub(viewport_height);
    let start = app.log_scroll.min(max_start);
    let end = (start + viewport_height).min(all_lines.len());
    let body = all_lines[start..end].join("\n");

    let paragraph = Paragraph::new(body).block(
        Block::default()
            .title("Run Log (up/down to scroll)")
            .borders(Borders::ALL),
    );
    frame.render_widget(paragraph, area);
}
