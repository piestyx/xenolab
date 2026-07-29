use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let items: Vec<ListItem> = app
        .actions()
        .iter()
        .map(|action| ListItem::new(action.label()))
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .title("Command Console")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut list_state = ListState::default();
    list_state.select(Some(app.menu_index));
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let measurement_text = if app.last_measurements.is_empty() {
        String::from("No measurements yet.")
    } else {
        app.last_measurements
            .iter()
            .map(|m| {
                format!(
                    "{}: {:.2} -> {:.2}",
                    m.node.stable_name(),
                    m.true_value,
                    m.measured_value
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    };
    let measurement = Paragraph::new(measurement_text).block(
        Block::default()
            .title("Last Measurement")
            .borders(Borders::ALL),
    );
    frame.render_widget(measurement, chunks[1]);
}
