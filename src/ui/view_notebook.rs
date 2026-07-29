use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::engine::notebook::{HypothesisDirection, ObservableVariable};
use crate::ui::app::{App, NotebookEditor, NotebookField};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let notebook = app.simulator.notebook();
    let mut lines = vec![format!(
        "NOTEBOOK    {} / {}    ({} slots remaining)",
        notebook.hypotheses().len(),
        notebook.capacity(),
        notebook.remaining_slots()
    )];

    if notebook.hypotheses().is_empty() {
        lines.push(String::new());
        lines.push("No hypotheses recorded.".to_string());
        lines.push("Record a theory about two observable variables.".to_string());
    } else {
        lines.push(String::new());
        for (index, hypothesis) in notebook.hypotheses().iter().enumerate() {
            let marker = if index == app.notebook_selected {
                ">"
            } else {
                " "
            };
            lines.push(format!(
                "{marker} {}. [{}] {}",
                index + 1,
                hypothesis.id.0,
                hypothesis.sentence()
            ));
        }
    }

    lines.push(String::new());
    if let Some(editor) = app.notebook_editor {
        lines.push(
            "EDITOR — Up/Down choose | Tab/Left/Right field | Enter confirm | Esc cancel"
                .to_string(),
        );
        lines.push(format_editor(editor));
    } else if let Some(id) = app.notebook_delete_confirmation {
        lines.push(format!(
            "Delete hypothesis [{}]? Enter confirm | Esc cancel",
            id.0
        ));
    } else if app.is_resolved() {
        lines.push("Run resolved: Notebook is read-only.".to_string());
    } else {
        lines.push("a add | e edit selected | d delete selected".to_string());
        lines.push("Notebook edits consume no actions, ticks, or contamination.".to_string());
    }
    if !app.status_message.is_empty() {
        lines.push(format!("Status: {}", app.status_message));
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().title("Notebook").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn format_editor(editor: NotebookEditor) -> String {
    let cause = ObservableVariable::ALL[editor.cause].label();
    let direction = match editor.direction {
        HypothesisDirection::Increases => "increases",
        HypothesisDirection::Decreases => "decreases",
    };
    let effect = ObservableVariable::ALL[editor.effect].label();
    format!(
        "Cause: {}{} | Direction: {}{} | Effect: {}{}{}",
        field_marker(editor.field, NotebookField::Cause),
        cause,
        field_marker(editor.field, NotebookField::Direction),
        direction,
        field_marker(editor.field, NotebookField::Effect),
        effect,
        if editor.field == NotebookField::Effect {
            " (Enter confirm)"
        } else {
            ""
        }
    )
}

fn field_marker(current: NotebookField, field: NotebookField) -> &'static str {
    if current == field {
        "[selected] "
    } else {
        ""
    }
}
