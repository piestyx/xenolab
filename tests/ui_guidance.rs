use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use xenolab::ui::app::{ActiveView, App, LogFilter};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn objective_guidance_and_action_metadata_are_player_facing() {
    let app = App::new(42);
    let progress = app.objective_progress_text();
    assert!(
        progress.contains("Plant stability")
            || progress.contains("Detox hold")
            || progress.contains("Collapse prevention")
    );
    assert!(app.actions()[0].description().contains("UV"));
    assert_eq!(
        app.actions()[8].measurement_category(),
        Some("population instrument")
    );
    assert!(App::terminal_size_supported(80, 24));
    assert!(!App::terminal_size_supported(79, 24));
    assert!(!App::terminal_size_supported(80, 23));
}

#[test]
fn repeat_last_action_uses_the_normal_engine_path_and_restart_clears_it() {
    let mut app = App::new(42);
    let before = app.actions_remaining();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(app.actions_remaining(), before - 1);
    app.handle_key(key(KeyCode::Char('x'))).unwrap();
    assert_eq!(app.actions_remaining(), before - 2);

    app.active_view = ActiveView::Log;
    app.handle_key(key(KeyCode::Char('m'))).unwrap();
    assert_eq!(app.log_filter, LogFilter::Measurements);
    app.handle_key(key(KeyCode::Char('a'))).unwrap();
    assert_eq!(app.log_filter, LogFilter::All);

    app.should_quit = false;
    app.handle_key(key(KeyCode::Char('?'))).unwrap();
    assert!(app.help_open);
    app.handle_key(key(KeyCode::Char('?'))).unwrap();
    assert!(!app.help_open);
}

#[test]
fn undersized_terminal_renders_resize_message_and_quit_remains_available() {
    let backend = TestBackend::new(79, 23);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new(42);
    terminal.draw(|frame| app.render(frame)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Resize required"));
    app.handle_key(key(KeyCode::Char('q'))).unwrap();
    assert!(app.should_quit);
}
