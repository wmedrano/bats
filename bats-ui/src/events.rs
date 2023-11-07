use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use log::debug;
use std::time::{Duration, Instant};

/// Poll for events.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct EventPoll {}

/// A user input event.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum Event {
    /// Nothing of significance happened.
    #[default]
    None,
    /// The up arrow key was pressed.
    Up,
    /// The down arrow key was pressed.
    Down,
    /// The left arrow key was pressed.
    Left,
    /// The right arrow key was pressed.
    Right,
    /// The back button (or esc) was pressed.
    Back,
    /// The enter key was pressed.
    Enter,
    /// A redraw was requested.
    Redraw,
}

impl EventPoll {
    /// Iterate over all events indefinitely.
    pub fn iter(&self) -> impl Iterator<Item = Result<Event>> {
        self.iter_with_timeout(None)
    }

    /// Iterate over all events but return `None` once `timeout` has been exceeded.
    ///
    /// If `timeout` is `None`, then there will be no time limit.
    fn iter_with_timeout(
        &self,
        timeout: impl Into<Option<Duration>>,
    ) -> impl Iterator<Item = Result<Event>> {
        let timeout = timeout.into();
        let deadline = timeout.map(|t| Instant::now() + t);
        std::iter::from_fn(move || -> Option<Result<Event>> {
            let timeout = deadline
                .map(|d| d.duration_since(Instant::now()))
                .unwrap_or(Duration::MAX);
            let is_ready = match crossterm::event::poll(timeout) {
                Ok(b) => b,
                Err(err) => return Some(Err(err.into())),
            };
            if !is_ready {
                return None;
            }
            let raw_event = match crossterm::event::read() {
                Ok(e) => e,
                Err(err) => return Some(Err(err.into())),
            };
            debug!("Encountered raw event {:?}", raw_event);
            let e = match raw_event {
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => return Some(Err(anyhow!("Exit with C-c requested."))),
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Up,
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Down,
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Left,
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Right,
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Back,
                crossterm::event::Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => Event::Enter,
                crossterm::event::Event::Resize(_, _) => Event::Redraw,
                _ => Event::None,
            };
            Some(Ok(e))
        })
    }
}
