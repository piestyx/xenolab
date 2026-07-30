use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::engine::repair::RepairTrack;
use crate::ui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let calibration = app.simulator.calibration_level();
    let containment = app.simulator.containment_level();
    let selected = if app.repair_selected == 0 {
        RepairTrack::Calibration
    } else {
        RepairTrack::Containment
    };
    let mut text = format!(
        "REPAIRS\n\nCredits: {} available\nEarned: {}\nSpent: {}\nMaximum earned: {}\n\n{}Calibration — Level {} / 2\n  Current: scan noise ×{:.2}\n  Next:    {}\n  Cost:    {}\n\n{}Containment — Level {} / 2\n  Current: contamination cost -{}\n  Next:    {}\n  Cost:    {}\n\n",
        app.simulator.credits_available(),
        app.simulator.credits_earned(),
        app.simulator.credits_spent(),
        app.simulator.max_research_credits(),
        marker(selected == RepairTrack::Calibration),
        calibration.level(),
        calibration.noise_multiplier(),
        calibration_next_text(calibration),
        cost_text(calibration.next_cost()),
        marker(selected == RepairTrack::Containment),
        containment.level(),
        containment.contamination_reduction(),
        containment_next_text(containment),
        cost_text(containment.next_cost()),
    );

    if let Some(track) = app.repair_confirmation {
        let (before, after, cost, effect) = match track {
            RepairTrack::Calibration => (
                calibration.level(),
                calibration.level() + 1,
                calibration.next_cost().unwrap_or(0),
                format!(
                    "future scan multiplier ×{:.2} → ×{:.2}",
                    calibration.noise_multiplier(),
                    calibration.advance().noise_multiplier()
                ),
            ),
            RepairTrack::Containment => (
                containment.level(),
                containment.level() + 1,
                containment.next_cost().unwrap_or(0),
                format!(
                    "future contamination reduction -{} → -{}",
                    containment.contamination_reduction(),
                    containment.advance().contamination_reduction()
                ),
            ),
        };
        text.push_str(&format!(
            "Purchase confirmation\n{}: Level {} → {}\nCost: {} credits\nAvailable after: {}\nEffect: {}\nEnter confirm | Esc cancel\n",
            track.label(),
            before,
            after,
            cost,
            app.simulator.credits_available().saturating_sub(cost),
            effect
        ));
    } else if app.is_resolved() {
        text.push_str("Run resolved: repairs are read-only.\n");
    } else {
        text.push_str("Up/Down select track | Enter purchase next level\n");
    }

    text.push_str("\nPurchase history\n");
    if app.simulator.repair_purchases().is_empty() {
        text.push_str("No repairs purchased.\n");
    } else {
        for purchase in app.simulator.repair_purchases() {
            text.push_str(&format!(
                "[{}] {} L{}→L{} spent={} remaining={} action={} tick={}\n",
                purchase.id.0,
                purchase.track.label(),
                purchase.level_before,
                purchase.level_after,
                purchase.credits_spent,
                purchase.credits_remaining,
                purchase.action_number,
                purchase.tick
            ));
        }
    }
    if !app.status_message.is_empty() {
        text.push_str(&format!("\nStatus: {}\n", app.status_message));
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Repairs").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn marker(selected: bool) -> &'static str {
    if selected {
        "> "
    } else {
        "  "
    }
}

fn cost_text(cost: Option<u32>) -> String {
    cost.map_or_else(
        || "maximum level".to_string(),
        |cost| format!("{} credits", cost),
    )
}

fn calibration_next_text(level: crate::engine::repair::CalibrationLevel) -> String {
    level.next_cost().map_or_else(
        || "maximum level".to_string(),
        |_| format!("scan noise ×{:.2}", level.advance().noise_multiplier()),
    )
}

fn containment_next_text(level: crate::engine::repair::ContainmentLevel) -> String {
    level.next_cost().map_or_else(
        || "maximum level".to_string(),
        |_| {
            format!(
                "contamination cost -{}",
                level.advance().contamination_reduction()
            )
        },
    )
}
