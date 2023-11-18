use std::io::Stdout;

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
use ratatui::{prelude::CrosstermBackend, style::Color, Terminal};

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
    bats_state: BatsState,
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
            bats_state,
        })
    }

    /// Run the UI.
    pub fn run(&mut self) -> Result<()> {
        #[derive(Copy, Clone)]
        enum MainMenuItem {
            Tracks,
            Metronome,
            Quit,
        }
        let menu_items = [
            MainMenuItem::Tracks,
            MainMenuItem::Metronome,
            MainMenuItem::Quit,
        ];
        let mut menu = SelectorMenu::new(
            "Main".to_string(),
            &menu_items,
            |i: &MainMenuItem| match i {
                MainMenuItem::Tracks => "Tracks".to_string(),
                MainMenuItem::Metronome => "Metronome".to_string(),
                MainMenuItem::Quit => "Quit".to_string(),
            },
        );
        loop {
            match menu.run(&self.event_poll, &mut self.terminal)? {
                Some(MainMenuItem::Tracks) => self.run_tracks()?,
                Some(MainMenuItem::Metronome) => self.run_metronome()?,
                Some(MainMenuItem::Quit) => return Ok(()),
                None => (),
            }
        }
    }

    /// Run the track menu page. This contains all tracks.
    fn run_tracks(&mut self) -> Result<()> {
        let tracks = self.bats_state.tracks_vec();
        let mut menu =
            SelectorMenu::new("Tracks".to_string(), tracks, |t: &TrackDetails| t.title());
        if let Some(track) = menu.run(&self.event_poll, &mut self.terminal)? {
            let track = self.bats_state.track_by_id(track.id).unwrap().clone();
            if track.plugin_metadata.name == "empty" {
                if let Some(plugin_builder) = Self::select_plugin(
                    format!("Select Plugin for {}", track.title()),
                    &self.event_poll,
                    &mut self.terminal,
                )? {
                    let plugin = plugin_builder.build(self.bats_state.sample_rate());
                    self.bats_state.set_plugin(track.id, plugin);
                }
            }
            self.run_single_track(track.id)?;
        };
        Ok(())
    }

    /// Run the metronome page.
    fn run_metronome(&mut self) -> Result<()> {
        let min_metronome_volume = 2f32.powi(-10);
        #[derive(Copy, Clone)]
        enum Item {
            Bpm,
            Volume,
            Back,
        }
        let mut menu = SelectorMenu::new(
            "Metronome".to_string(),
            [Item::Bpm, Item::Volume, Item::Back],
            |i: &Item| match i {
                Item::Bpm => format!("BPM: {bpm}", bpm = self.bats_state.bpm()),
                Item::Volume => {
                    format!(
                        "Volume: {volume}",
                        volume = ParamType::Decibel.formatted(self.bats_state.metronome_volume())
                    )
                }
                Item::Back => "Back".to_string(),
            },
        )
        .with_extra_event_handler(|event, selected| match (event, selected) {
            (events::Event::Left, Item::Volume) => {
                self.bats_state.modify_metronome(|v| {
                    if v <= min_metronome_volume {
                        0.0
                    } else {
                        v / 2f32.sqrt()
                    }
                });
                MenuAction::Redraw
            }
            (events::Event::Right, Item::Volume) => {
                self.bats_state.modify_metronome(|v| {
                    if v < min_metronome_volume {
                        min_metronome_volume
                    } else {
                        v * 2f32.sqrt()
                    }
                });
                MenuAction::Redraw
            }
            (events::Event::Left, Item::Bpm) => {
                self.bats_state.modify_bpm(|v| v - 1.0);
                MenuAction::Redraw
            }
            (events::Event::Right, Item::Bpm) => {
                self.bats_state.modify_bpm(|v| v + 1.0);
                MenuAction::Redraw
            }
            _ => MenuAction::None,
        });
        while let Some(item) = menu.run(&self.event_poll, &mut self.terminal)? {
            match item {
                Item::Bpm => (),
                Item::Volume => (),
                Item::Back => return Ok(()),
            }
        }
        Ok(())
    }

    /// Run the page for a single track. This has links to other pages for the track such as
    /// changing the plugin and adjusting the params.
    fn run_single_track(&mut self, track_id: usize) -> Result<()> {
        self.bats_state.set_armed(track_id);
        #[derive(Copy, Clone)]
        enum TrackMenuItem {
            ChangeVolume,
            ChangePlugin,
            Params,
            ClearSequence,
        }
        let menu_items = [
            TrackMenuItem::ChangeVolume,
            TrackMenuItem::ChangePlugin,
            TrackMenuItem::Params,
            TrackMenuItem::ClearSequence,
        ];
        let mut menu =
            SelectorMenu::new("".to_string(), &menu_items, |i: &TrackMenuItem| match i {
                TrackMenuItem::ChangeVolume => {
                    format!(
                        "Volume: {volume}",
                        volume = ParamType::Decibel
                            .formatted(self.bats_state.track_by_id(track_id).unwrap().volume)
                    )
                }
                TrackMenuItem::ChangePlugin => "Change Plugin".to_string(),
                TrackMenuItem::Params => "Params".to_string(),
                TrackMenuItem::ClearSequence => "Clear Sequence".to_string(),
            })
            .with_extra_event_handler(|event, action| match (action, event) {
                (TrackMenuItem::ChangeVolume, events::Event::Left) => {
                    self.bats_state
                        .modify_track_volume(track_id, |v| v.volume / 1.05);
                    MenuAction::Redraw
                }
                (TrackMenuItem::ChangeVolume, events::Event::Right) => {
                    self.bats_state
                        .modify_track_volume(track_id, |v| v.volume * 1.05);
                    MenuAction::Redraw
                }
                _ => MenuAction::None,
            });
        loop {
            menu.set_title(format!(
                "Track - {}",
                self.bats_state.track_by_id(track_id).unwrap().title()
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
                            self.bats_state.track_by_id(track_id).unwrap().title()
                        ),
                        &self.event_poll,
                        &mut self.terminal,
                    ) {
                        let plugin = b.build(self.bats_state.sample_rate());
                        self.bats_state.set_plugin(track_id, plugin);
                    }
                }
                TrackMenuItem::ChangeVolume => (),
                TrackMenuItem::Params => Self::edit_params(
                    &self.event_poll,
                    &mut self.terminal,
                    &self.bats_state,
                    track_id,
                )?,
                TrackMenuItem::ClearSequence => self.bats_state.set_sequence(track_id, Vec::new()),
            }
        }
    }

    /// Select a plugin and return it. If the selection is canceled, then `Ok(None)` is returned.
    fn select_plugin(
        title: String,
        event_poll: &EventPoll,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<Option<PluginBuilder>> {
        let mut menu = SelectorMenu::new(title, PluginBuilder::ALL, |b: &PluginBuilder| {
            b.name().to_string()
        });
        menu.run(event_poll, terminal)
    }

    /// Edit the params for the track with `track_id`.
    fn edit_params(
        event_poll: &EventPoll,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        bats_state: &BatsState,
        track_id: usize,
    ) -> Result<()> {
        let track = bats_state.track_by_id(track_id).unwrap().clone();
        let title = format!("{} Params", track.title());
        let mut menu = SelectorMenu::new(title, &track.plugin_metadata.params, |p: &Param| {
            let value = bats_state
                .track_by_id(track_id)
                .unwrap()
                .params
                .get(&p.id)
                .copied()
                .unwrap_or(0.0);
            format!(
                "{name}: {value}",
                name = p.name,
                value = p.param_type.formatted(value),
            )
        })
        .with_extra_event_handler(|event, param| match event {
            events::Event::Left => {
                bats_state.modify_param(track_id, param.id, |v| v / 1.05);
                MenuAction::Redraw
            }
            events::Event::Right => {
                bats_state.modify_param(track_id, param.id, |v| v * 1.05);
                MenuAction::Redraw
            }
            _ => MenuAction::None,
        })
        .with_color(Color::Blue);
        menu.run(event_poll, terminal)?;
        Ok(())
    }
}
