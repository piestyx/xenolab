use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::engine::ids::NodeId;
use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut items = vec![
        ListItem::new(format!("Objective: {}", app.objective.label())).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        ListItem::new(format!("Tick: {}", app.simulator.tick_index())),
        ListItem::new(format!(
            "Contamination: {:.2}",
            app.simulator.contamination()
        )),
        ListItem::new(format!("Status: {}", app.status_message)),
        ListItem::new(""),
    ];

    for node in NodeId::ALL {
        let value = app.simulator.state().get(node);
        items.push(ListItem::new(format!(
            "{:<14} {:>6.2}",
            node.stable_name(),
            value
        )));
    }

    let list = List::new(items).block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(list, area);
}
