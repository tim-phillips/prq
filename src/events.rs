use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

/// Poll the terminal for an input event, returning Tick if nothing arrives
/// within `timeout`. Resize events are swallowed (redraw happens anyway).
pub fn poll(timeout: Duration) -> Result<AppEvent> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(k) if k.kind == crossterm::event::KeyEventKind::Press => {
                return Ok(AppEvent::Key(k));
            }
            Event::Mouse(m) => return Ok(AppEvent::Mouse(m)),
            _ => return Ok(AppEvent::Tick),
        }
    }
    Ok(AppEvent::Tick)
}
