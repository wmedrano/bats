use std::{cell::RefCell, io::Stdout};

use anyhow::Result;
use bats_async::CommandSender;
use bats_lib::{
    plugin::metadata::{Param, ParamType},
    Bats,
};
use bats_state::{BatsState, TrackDetails};
use events::EventPoll;
use log::info;
use menu::{Menu, MenuAction, SelectorMenu};
use plugin_factory::PluginBuilder;
use ratatui::{prelude::CrosstermBackend, Terminal};

pub mod bats_state;
pub mod events;
pub mod menu;
pub mod plugin_factory;
pub mod selector;

/// Runs the Ui.
pub struct Ui {
    /// The backing terminal.
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// The object to poll events from.
    event_poll: EventPoll,
    /// Contains bats related state information.
    bats_state: RefCell<BatsState>,
}

impl Ui {
    /// Create a new `Ui`.
    pub fn new(bats: &Bats, commands: CommandSender) -> Result<Ui> {
        let bats_state = BatsState::new(bats, commands);
        // Initialize the terminal user interface.
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        terminal.hide_cursor()?;
        terminal.clear()?;
        info!("Initialized UI.");
        Ok(Ui {
            terminal,
            event_poll: EventPoll {},
            bats_state: RefCell::new(bats_state),
        })
    }

    /// Run the UI.
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.run_track()?;
        }
    }

    /// Run the track menu page. This contains all tracks.
    fn run_track(&mut self) -> Result<()> {
        let tracks = self.bats_state.borrow().tracks().cloned().collect();
        let mut menu =
            SelectorMenu::new("Tracks".to_string(), tracks, |t: &TrackDetails| t.title());
        if let Some(track) = menu.run(&self.event_poll, &mut self.terminal)? {
            let track = self
                .bats_state
                .borrow()
                .tracks()
                .find(|t| t.id == track.id)
                .unwrap()
                .clone();
            if track.plugin_metadata.name == "empty" {
                if let Some(plugin_builder) = Self::select_plugin(
                    format!("Select Plugin for {}", track.title()),
                    &self.event_poll,
                    &mut self.terminal,
                )? {
                    let plugin = plugin_builder.build(self.bats_state.borrow().sample_rate);
                    self.bats_state.borrow_mut().set_plugin(track.id, plugin);
                }
            } else {
                self.run_single_track(track.id)?;
            }
        };
        Ok(())
    }

    /// Run the page for a single track. This has links to other pages for the track such as
    /// changing the plugin and adjusting the params.
    fn run_single_track(&mut self, track_id: usize) -> Result<()> {
        #[derive(Copy, Clone)]
        enum TrackMenuItem {
            ChangeVolume,
            ChangePlugin,
            Params,
        }
        let menu_items = vec![
            TrackMenuItem::ChangeVolume,
            TrackMenuItem::ChangePlugin,
            TrackMenuItem::Params,
        ];
        let mut menu = SelectorMenu::new("".to_string(), menu_items, |i: &TrackMenuItem| match i {
            TrackMenuItem::ChangeVolume => {
                format!(
                    "Volume: {volume}",
                    volume = ParamType::Decibel.formatted(
                        self.bats_state
                            .borrow()
                            .track_by_id(track_id)
                            .unwrap()
                            .volume
                    )
                )
            }
            TrackMenuItem::ChangePlugin => "Change Plugin".to_string(),
            TrackMenuItem::Params => "Params".to_string(),
        })
        .with_extra_event_handler(|event, action| match (action, event) {
            (TrackMenuItem::ChangeVolume, events::Event::Left) => {
                self.bats_state
                    .borrow_mut()
                    .modify_track_volume(track_id, |v| v.volume / 1.05);
                MenuAction::Redraw
            }
            (TrackMenuItem::ChangeVolume, events::Event::Right) => {
                self.bats_state
                    .borrow_mut()
                    .modify_track_volume(track_id, |v| v.volume * 1.05);
                MenuAction::Redraw
            }
            _ => MenuAction::None,
        });
        loop {
            menu.set_title(format!(
                "Track - {}",
                self.bats_state
                    .borrow()
                    .track_by_id(track_id)
                    .unwrap()
                    .title()
            ));
            let selected = match menu.run(&self.event_poll, &mut self.terminal)? {
                Some(s) => s,
                None => return Ok(()),
            };
            match selected {
                TrackMenuItem::ChangePlugin => {
                    if let Ok(Some(b)) = Self::select_plugin(
                        format!(
                            "Change Plugin for {}",
                            self.bats_state
                                .borrow()
                                .track_by_id(track_id)
                                .unwrap()
                                .title()
                        ),
                        &self.event_poll,
                        &mut self.terminal,
                    ) {
                        let plugin = b.build(self.bats_state.borrow().sample_rate);
                        self.bats_state.borrow_mut().set_plugin(track_id, plugin);
                    }
                }
                TrackMenuItem::ChangeVolume => (),
                TrackMenuItem::Params => Self::edit_params(
                    &self.event_poll,
                    &mut self.terminal,
                    &self.bats_state,
                    track_id,
                )?,
            }
        }
    }

    /// Select a plugin and return it. If the selection is canceled, then `Ok(None)` is returned.
    fn select_plugin(
        title: String,
        event_poll: &EventPoll,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<Option<PluginBuilder>> {
        let mut menu =
            SelectorMenu::new(title, PluginBuilder::ALL.to_vec(), |b: &PluginBuilder| {
                b.name().to_string()
            });
        menu.run(event_poll, terminal)
    }

    /// Edit the params for the track with `track_id`.
    pub fn edit_params(
        event_poll: &EventPoll,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        bats_state: &RefCell<BatsState>,
        track_id: usize,
    ) -> Result<()> {
        let track = bats_state.borrow().track_by_id(track_id).unwrap().clone();
        let title = format!("{} Params", track.title());
        let params = track.plugin_metadata.params.to_vec();
        let mut menu = SelectorMenu::new(title, params, |p: &Param| {
            let value = bats_state
                .borrow()
                .track_by_id(track_id)
                .unwrap()
                .params
                .get(&p.id)
                .copied()
                .unwrap_or(0.0);
            format!("{name}: {value}", name = p.name, value = p.formatted(value),)
        })
        .with_extra_event_handler(|event, param| match event {
            events::Event::Left => {
                bats_state
                    .borrow_mut()
                    .modify_param(track_id, param.id, |v| v / 1.05);
                MenuAction::Redraw
            }
            events::Event::Right => {
                bats_state
                    .borrow_mut()
                    .modify_param(track_id, param.id, |v| v * 1.05);
                MenuAction::Redraw
            }
            _ => MenuAction::None,
        });
        menu.run(event_poll, terminal)?;
        Ok(())
    }
}
