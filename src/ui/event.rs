use std::io;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};

pub fn read_key(timeout: Duration) -> io::Result<Option<KeyEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }

    match event::read()? {
        CrosstermEvent::Key(key) => Ok(Some(key)),
        _ => Ok(None),
    }
}
