use anyhow::Result;
use ratatui::{
    prelude::Alignment,
    style::{Color, Style},
    widgets, Frame, Terminal,
};

use crate::{
    events::{Event, EventPoll},
    selector::Selector,
};

/// A menu action to perform.
pub enum MenuAction<T> {
    /// Do not do anything and keep the menu going.
    None,
    /// Select the item from the menu.
    Select(T),
    /// Exit the menu with no selection.
    Exit,
    /// Redraw the menu.
    Redraw,
}

/// A trait for implementing a menu.
pub trait Menu {
    /// The item that the menu will select for.
    type Item;

    /// Handle a user event and return the menu action that should be performed.
    fn handle_event(&mut self, event: Event) -> Result<MenuAction<Self::Item>>;

    /// Draw the menu. Typically called at the start of run and whenever redraw is requested.
    fn draw(&mut self, frame: &mut Frame);

    /// Run the menu. Typically, the default implementation should be used as this is the main
    /// helper the trait provides.
    fn run<T: ratatui::prelude::Backend>(
        &mut self,
        event_poll: &EventPoll,
        terminal: &mut Terminal<T>,
    ) -> Result<Option<Self::Item>> {
        terminal.draw(|f| self.draw(f))?;
        for event_or_err in event_poll.iter() {
            let event = event_or_err?;
            match self.handle_event(event)? {
                MenuAction::None => (),
                MenuAction::Select(item) => return Ok(Some(item)),
                MenuAction::Exit => return Ok(None),
                MenuAction::Redraw => {
                    terminal.draw(|f| self.draw(f))?;
                }
            }
        }
        unreachable!("EventPoll should not run out of events.");
    }
}

/// A function that handles events for a selector.
type SelectorEventHandler<'a, T> = dyn 'a + FnMut(Event, &T) -> MenuAction<T>;

/// A basic menu that selects an item of type `T`.
pub struct SelectorMenu<'a, T, F, A: AsRef<[T]>> {
    title: String,
    selection: Selector<T, A>,
    formatter: F,
    extra_event_handler: Box<SelectorEventHandler<'a, T>>,
}

impl<'a, T, F, A: AsRef<[T]>> SelectorMenu<'a, T, F, A> {
    /// Create a new menu with the given title and items. `formatter` is used to convert an item of
    /// type `T` into a human readable menu option.
    pub fn new(title: String, items: A, formatter: F) -> SelectorMenu<'static, T, F, A> {
        SelectorMenu {
            title,
            selection: Selector::new(items),
            formatter,
            extra_event_handler: Box::new(|_, _| MenuAction::None),
        }
    }

    /// Add an extra handler. Allows doing extra actions with unused user input. Typically, the only
    /// user input that `SelectorMenu` uses are the up/down arrow keys, exit, and enter.
    pub fn with_extra_event_handler<'b>(
        self,
        handler: impl 'b + FnMut(Event, &T) -> MenuAction<T>,
    ) -> SelectorMenu<'b, T, F, A> {
        SelectorMenu {
            title: self.title,
            selection: self.selection,
            formatter: self.formatter,
            extra_event_handler: Box::new(handler),
        }
    }

    /// Set the title.
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

impl<'a, T: Clone, F: Fn(&T) -> String, A: AsRef<[T]>> Menu for SelectorMenu<'a, T, F, A> {
    type Item = T;

    fn handle_event(&mut self, event: Event) -> Result<MenuAction<Self::Item>> {
        let action = match event {
            Event::Up => {
                self.selection.select_by(-1);
                MenuAction::Redraw
            }
            Event::Down => {
                self.selection.select_by(1);
                MenuAction::Redraw
            }
            Event::Back => MenuAction::Exit,
            Event::Enter => MenuAction::Select(self.selection.selected().clone()),
            Event::Redraw => MenuAction::Redraw,
            other => (self.extra_event_handler)(other, self.selection.selected()),
        };
        Ok(action)
    }

    fn draw(&mut self, frame: &mut Frame) {
        let items: Vec<_> = self
            .selection
            .iter()
            .map(|(selected, item)| {
                let selected = if selected { ">>" } else { "  " };
                let item_text = (self.formatter)(item);
                widgets::ListItem::new(format!("{selected} {item_text}"))
            })
            .collect();
        frame.render_widget(
            widgets::List::new(items)
                .block(
                    widgets::Block::default()
                        .title(self.title.as_str())
                        .title_alignment(Alignment::Center)
                        .borders(widgets::Borders::ALL)
                        .border_type(widgets::BorderType::Rounded),
                )
                .style(Style::default().fg(Color::White).bg(Color::Black)),
            frame.size(),
        )
    }
}
