use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::ui::app::App;
use crate::ui::event;
use crate::ui::UiError;

pub fn run_app(mut app: App) -> Result<(), UiError> {
    let mut terminal = TerminalSession::enter()?;

    while !app.should_quit {
        terminal.terminal.draw(|frame| app.render(frame))?;
        if let Some(key) = event::read_key(Duration::from_millis(100))? {
            app.handle_key(key)?;
        }
    }

    terminal.exit()?;
    Ok(())
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    in_alt_screen: bool,
}

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        stdout.execute(crossterm::event::EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            in_alt_screen: true,
        })
    }

    fn exit(&mut self) -> io::Result<()> {
        if self.in_alt_screen {
            disable_raw_mode()?;
            execute!(
                self.terminal.backend_mut(),
                crossterm::event::DisableMouseCapture,
                LeaveAlternateScreen
            )?;
            self.terminal.show_cursor()?;
            self.in_alt_screen = false;
        }
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}
