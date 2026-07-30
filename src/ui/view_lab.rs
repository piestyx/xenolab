use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::engine::ids::NodeId;
use crate::ui::app::App;

pub fn render_lab(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(area);

    render_top_row(f, app, rows[0]);
    render_main_columns(f, app, rows[1]);
    render_last_result(f, app, rows[2]);
}

fn render_top_row(f: &mut Frame, app: &App, area: Rect) {
    let summary = format!(
        "seed={} | objective={} | tick={} | contamination={:.2} | {}",
        app.seed,
        app.objective.label(),
        app.simulator.tick_index(),
        app.simulator.contamination(),
        app.status_message
    );

    let paragraph = Paragraph::new(summary)
        .block(Block::default().title("Lab").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_main_columns(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(area);

    let status_items = vec![
        ListItem::new(format!("Objective: {}", app.objective.label())).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        ListItem::new(format!(
            "{} ({})",
            app.objective_summary_short(),
            app.objective_progress_text()
        )),
        ListItem::new(format!(
            "Actions remaining: {} / {}",
            app.actions_remaining(),
            app.action_limit()
        )),
        ListItem::new(format!(
            "Credits: {} available (earned {}, spent {})",
            app.simulator.credits_available(),
            app.simulator.credits_earned(),
            app.simulator.credits_spent()
        )),
        ListItem::new(format!(
            "Repairs: Calibration L{} | Containment L{}",
            app.simulator.calibration_level().level(),
            app.simulator.containment_level().level()
        )),
        ListItem::new(format!(
            "Publications: {} / {}",
            app.simulator.publications().len(),
            app.simulator.publication_limit()
        )),
        ListItem::new(format!(
            "Contamination: {:.0} / 40 ({})",
            app.simulator.contamination(),
            app.contamination_level().label()
        )),
        ListItem::new(format!(
            "Scan noise: {:.2}x (cal ×{:.2})",
            app.contamination_noise_multiplier() * app.simulator.calibration_multiplier(),
            app.simulator.calibration_multiplier()
        )),
        ListItem::new(format!(
            "Next threshold: {}",
            app.contamination_next_threshold()
                .map_or_else(|| "none".to_string(), |threshold| threshold.to_string())
        )),
        ListItem::new(""),
        ListItem::new(format!(
            "Plant: {:.2}",
            app.simulator.state().get(NodeId::PlantPop)
        )),
        ListItem::new(format!(
            "Fungus: {:.2}",
            app.simulator.state().get(NodeId::FungusLoad)
        )),
        ListItem::new(format!(
            "Bacteria: {:.2}",
            app.simulator.state().get(NodeId::BacteriaPop)
        )),
        ListItem::new(format!(
            "Toxin: {:.2}",
            app.simulator.state().get(NodeId::Toxin)
        )),
        ListItem::new(format!(
            "Nutrient: {:.2}",
            app.simulator.state().get(NodeId::Nutrient)
        )),
    ];
    let status =
        List::new(status_items).block(Block::default().title("Status").borders(Borders::ALL));
    f.render_widget(status, cols[0]);

    let metrics = [
        (NodeId::UvLevel, "uv"),
        (NodeId::PlantPop, "plant"),
        (NodeId::FungusLoad, "fungus"),
        (NodeId::BacteriaPop, "bacteria"),
        (NodeId::Toxin, "toxin"),
        (NodeId::Nutrient, "nutrient"),
    ];
    let metric_items: Vec<ListItem> = metrics
        .iter()
        .map(|(node, label)| {
            let value = app.simulator.state().get(*node);
            let delta = app.delta_for(*node);
            ListItem::new(format!(
                "{:<9} {:>6.1} ({:+.0})",
                label,
                value,
                delta.round()
            ))
        })
        .collect();
    let state_block = List::new(metric_items).block(
        Block::default()
            .title("World State (value + delta)")
            .borders(Borders::ALL),
    );
    f.render_widget(state_block, cols[1]);

    let items: Vec<ListItem> = app
        .actions()
        .iter()
        .map(|action| {
            let intervention = action.to_intervention();
            let base = intervention.contamination_cost();
            let effective = app.simulator.effective_contamination_cost(&intervention);
            let cost = if base == effective {
                format!("c+{effective}")
            } else {
                format!("c+{effective} (base +{base})")
            };
            ListItem::new(format!("{} {cost}", action.label()))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().title("Actions").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut list_state = ListState::default();
    list_state.select(Some(app.menu_index));
    f.render_stateful_widget(list, cols[2], &mut list_state);
}

fn render_last_result(f: &mut Frame, app: &App, area: Rect) {
    let action_line = match &app.last_event_summary {
        Some(summary) => format!("Last action: {summary}"),
        None => String::from("Last action: none"),
    };

    let measurement_line = if app.last_measurements.is_empty() {
        String::from("Measurement: none")
    } else {
        let joined = app
            .last_measurements
            .iter()
            .map(|m| {
                format!(
                    "{} {:.1}->{:.1} (sigma {:.2})",
                    m.node.stable_name(),
                    m.true_value,
                    m.measured_value,
                    m.effective_sigma
                )
            })
            .collect::<Vec<String>>()
            .join(", ");
        format!("Measurement: {joined}")
    };

    let recent: Vec<String> = app
        .simulator
        .events()
        .iter()
        .rev()
        .take(4)
        .map(|event| {
            format!(
                "t{} {} meas={} c={:.1}",
                event.tick_index,
                event.intervention.label(),
                event.measurements.len(),
                event.contamination
            )
        })
        .collect();
    let recent_line = if recent.is_empty() {
        String::from("Recent: none")
    } else {
        format!("Recent:\n{}", recent.join("\n"))
    };

    let text = format!("{action_line}\n{measurement_line}\n{recent_line}");
    let panel = Paragraph::new(text)
        .block(Block::default().title("Last Result").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(panel, area);
}
