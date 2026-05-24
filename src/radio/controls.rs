use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEvent {
    TogglePause,
    Stop,
}

pub struct RawMode;

impl RawMode {
    pub fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

pub fn spawn_control_task() -> mpsc::Receiver<ControlEvent> {
    let (tx, rx) = mpsc::channel(8);
    std::thread::spawn(move || {
        loop {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => {
                        if let Some(control) = map_key(key) {
                            if tx.blocking_send(control).is_err() || control == ControlEvent::Stop {
                                break;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(error = %error, "failed to read terminal event");
                        break;
                    }
                },
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(error = %error, "failed to poll terminal event");
                    break;
                }
            }
        }
    });
    rx
}

fn map_key(key: KeyEvent) -> Option<ControlEvent> {
    match key.code {
        KeyCode::Char('q') => Some(ControlEvent::Stop),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(ControlEvent::Stop)
        }
        KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') => {
            Some(ControlEvent::TogglePause)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ruv_controls() {
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(ControlEvent::Stop)
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(ControlEvent::TogglePause)
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(ControlEvent::TogglePause)
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(ControlEvent::Stop)
        );
    }
}
