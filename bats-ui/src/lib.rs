use anyhow::{anyhow, Result};
use bats_async::CommandSender;
use bats_lib::{
    plugin::{metadata::ParamType, toof::Toof, BatsInstrument},
    Bats,
};
use bats_state::{BatsState, TrackDetails};
use colors::ColorScheme;
use frame_counter::FrameCounter;
use log::{info, warn};
use sdl2::{event::Event, keyboard::Keycode, render::Canvas, video::Window, EventPump};
use selection::MenuSelection;
use text::TextRenderer;

use crate::param::ParamFormatter;

pub mod bats_state;
pub mod colors;
pub mod frame_counter;
pub mod param;
pub mod selection;
pub mod text;

/// Options the user has requested for the window.
#[derive(Debug, PartialEq)]
enum ProgramRequest {
    /// The user has (implicitly) requested to continue on the
    /// program.
    Continue,
    /// The user has requested to exit the program.
    Exit,
}

/// The current page.
#[derive(Copy, Clone, Debug)]
enum Page {
    /// The main menu.
    MainMenu,
    /// The tracks menu.
    TracksMenu,
    /// The page for a single track.
    Track { selection: MenuSelection },
    /// The metronome.
    Metronome,
}

/// Runs the Ui.
pub struct Ui {
    /// The current page.
    page: Page,
    /// The state for the track menu.
    tracks_menu: MenuSelection,
    /// The canvas to render items onto.
    canvas: Canvas<Window>,
    /// An iterator over events.
    event_pump: EventPump,
    /// The color scheme to use.
    color_scheme: ColorScheme,
    /// The text renderer.
    text_renderer: TextRenderer,
    /// Frame stats.
    frame_counter: FrameCounter,
    /// Contains bats related state information.
    bats_state: BatsState,
}

fn selected_track<'a>(
    track_selection: MenuSelection,
    tracks: impl Iterator<Item = &'a TrackDetails>,
) -> Option<TrackDetails> {
    track_selection.selection(std::iter::once(None).chain(tracks.cloned().map(Some)))?
}

impl Ui {
    pub const MIN_TRACK_VOLUME: f32 = 0.015625;
    pub const MAX_TRACK_VOLUME: f32 = 4.0;

    /// Create a new `Ui`.
    pub fn new(bats: &Bats, commands: CommandSender) -> Result<Ui> {
        let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
        let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
        let window = video_subsystem
            .window("bats", 320, 240)
            .opengl()
            .resizable()
            .build()
            .map_err(anyhow::Error::msg)?;
        let canvas = window
            .into_canvas()
            .index(find_sdl_gl_driver()?)
            .present_vsync()
            .build()
            .map_err(anyhow::Error::msg)?;
        let event_iter = sdl_context.event_pump().map_err(anyhow::Error::msg)?;
        let color_scheme = ColorScheme::default();
        let text_renderer = TextRenderer::new(&canvas)?;
        let bats_state = BatsState::new(bats, commands);
        info!("UI initialized.");
        Ok(Ui {
            page: Page::MainMenu,
            tracks_menu: MenuSelection { selected_idx: 0 },
            canvas,
            event_pump: event_iter,
            color_scheme,
            text_renderer,
            frame_counter: FrameCounter::new(),
            bats_state,
        })
    }

    /// Run the UI. This function will keep running until the user
    /// requests an exit.
    pub fn run(&mut self) -> Result<()> {
        while self.handle_events() == ProgramRequest::Continue {
            let frame_number = self.frame_counter.next_frame();
            self.render(frame_number);
        }
        Ok(())
    }

    /// Handle all events in the queue.
    ///
    /// Returns `ProgramRequest::Exit` if the user has requested that
    /// the program be exited.
    fn handle_events(&mut self) -> ProgramRequest {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return ProgramRequest::Exit,
                Event::KeyDown {
                    keycode: Some(k), ..
                } => match (self.page, k) {
                    (_, Keycode::Escape) => self.page = Page::MainMenu,
                    (Page::TracksMenu, Keycode::Up) => self
                        .tracks_menu
                        .move_selection(-1, self.bats_state.tracks().count() + 1),
                    (Page::TracksMenu, Keycode::Down) => self
                        .tracks_menu
                        .move_selection(1, self.bats_state.tracks().count() + 1),
                    (Page::TracksMenu, Keycode::Return) => {
                        match selected_track(self.tracks_menu, self.bats_state.tracks()) {
                            Some(t) => {
                                self.bats_state.set_armed(Some(t.id));
                                self.page = Page::Track {
                                    selection: MenuSelection::default(),
                                };
                            }
                            None => {
                                let id = self
                                    .bats_state
                                    .add_plugin(Toof::new(self.bats_state.sample_rate))
                                    .id;
                                self.bats_state.set_armed(Some(id));
                            }
                        }
                    }
                    (Page::Track { mut selection }, Keycode::Up) => {
                        selection.move_selection(
                            -1,
                            self.bats_state
                                .selected_track()
                                .map(|t| t.plugin_metadata.params.len())
                                .unwrap_or(0),
                        );
                        self.page = Page::Track { selection };
                    }
                    (Page::Track { mut selection }, Keycode::Down) => {
                        selection.move_selection(
                            1,
                            self.bats_state
                                .selected_track()
                                .map(|t| t.plugin_metadata.params.len())
                                .unwrap_or(0),
                        );
                        self.page = Page::Track { selection };
                    }
                    _ => (),
                },
                Event::TextInput { text, .. } => match (self.page, text.as_str()) {
                    (_, "M") => self.bats_state.toggle_metronome(),
                    (_, "m") => self.page = Page::Metronome,
                    (_, "t") => self.page = Page::TracksMenu,
                    (_, "q") => return ProgramRequest::Exit,
                    (Page::Metronome, "+") => self.bats_state.set_bpm(self.bats_state.bpm() + 0.5),
                    (Page::Metronome, "-") => self.bats_state.set_bpm(self.bats_state.bpm() - 0.5),
                    (Page::TracksMenu, "+") => {
                        if let Some(t) = selected_track(self.tracks_menu, self.bats_state.tracks())
                        {
                            self.bats_state.set_track_volume(
                                t.id,
                                (t.volume * 1.05)
                                    .clamp(Self::MIN_TRACK_VOLUME, Self::MAX_TRACK_VOLUME),
                            );
                        }
                    }
                    (Page::TracksMenu, "-") => {
                        if let Some(t) = selected_track(self.tracks_menu, self.bats_state.tracks())
                        {
                            self.bats_state.set_track_volume(
                                t.id,
                                (t.volume / 1.05)
                                    .clamp(Self::MIN_TRACK_VOLUME, Self::MAX_TRACK_VOLUME),
                            );
                        }
                    }
                    (Page::Track { selection }, "+") => {
                        let track_id = self.bats_state.selected_track().unwrap().id;
                        let params = self
                            .bats_state
                            .selected_track()
                            .unwrap()
                            .plugin_metadata
                            .params;
                        let param = params[selection.selected_idx];
                        let value = (self.bats_state.param(track_id, param.id) * 1.05)
                            .clamp(param.min_value, param.max_value);
                        self.bats_state.set_param(track_id, param.id, value);
                    }
                    (Page::Track { selection }, "-") => {
                        let track_id = self.bats_state.selected_track().unwrap().id;
                        let params = self
                            .bats_state
                            .selected_track()
                            .unwrap()
                            .plugin_metadata
                            .params;
                        let param = params[selection.selected_idx];
                        let value = (self.bats_state.param(track_id, param.id) / 1.05)
                            .clamp(param.min_value, param.max_value);
                        self.bats_state.set_param(track_id, param.id, value);
                    }
                    x => warn!("Unhandled input {:?}", x),
                },
                _ => (),
            }
        }
        ProgramRequest::Continue
    }

    /// Render a new frame and present it. It should be automatically
    /// synchronized and frame limitted.
    fn render(&mut self, frame_number: usize) {
        self.canvas.set_draw_color(self.color_scheme.background);
        self.canvas.clear();
        if frame_number % 256 == 0 {
            self.text_renderer.clear_unused_cache();
        }
        match self.page {
            Page::MainMenu => self.render_main_menu(),
            Page::TracksMenu => self.render_tracks(),
            Page::Track { selection } => self.render_track(selection),
            Page::Metronome => self.render_metronome_menu(),
        }
        self.text_renderer
            .render(
                &mut self.canvas,
                self.frame_counter.fps().to_string(),
                self.color_scheme.middleground,
                (232, 220),
            )
            .unwrap();
        self.canvas.present();
    }

    /// Render the main menu.
    fn render_main_menu(&mut self) {
        let items = ["m - Metronome", "t - Tracks", "q - Quit"];
        self.text_renderer
            .render_menu(
                &mut self.canvas,
                self.color_scheme.foreground,
                "Main Menu".to_string(),
                items.iter().map(|i| i.to_string()),
                None,
            )
            .unwrap();
    }

    /// Render the tracks menu.
    fn render_tracks(&mut self) {
        let items =
            std::iter::once("Add Track".to_string()).chain(self.bats_state.tracks().map(|t| {
                format!(
                    "{} {}",
                    t.plugin_metadata.name,
                    ParamFormatter::from((ParamType::Percent, t.volume))
                )
            }));
        self.text_renderer
            .render_menu(
                &mut self.canvas,
                self.color_scheme.foreground,
                "Tracks".to_string(),
                items,
                Some(self.tracks_menu.selected_idx),
            )
            .unwrap();
    }

    /// Render a track.
    fn render_track(&mut self, selection: MenuSelection) {
        let armed_track = self.bats_state.armed();
        let track = self
            .bats_state
            .tracks()
            .find(|t| Some(t.id) == armed_track)
            .cloned()
            .unwrap_or_default();
        let items = track.plugin_metadata.params.iter().map(|p| {
            let value = self.bats_state.param(track.id, p.id);
            let param: ParamFormatter = (p.param_type, value).into();
            format!("{}: {}", p.name, param)
        });
        self.text_renderer
            .render_menu(
                &mut self.canvas,
                self.color_scheme.foreground,
                format!("Track: {}", track.plugin_metadata.name),
                items,
                Some(selection.selected_idx),
            )
            .unwrap();
    }

    /// Render the metronome menu.
    fn render_metronome_menu(&mut self) {
        let items = [
            format!("BPM (+/-): {}", self.bats_state.bpm_text()),
            "M - Toggle Metronome".to_string(),
        ]
        .into_iter();
        self.text_renderer
            .render_menu(
                &mut self.canvas,
                self.color_scheme.foreground,
                "Metronome".to_string(),
                items,
                None,
            )
            .unwrap();
    }
}

/// Find the OpenGL driver index.
///
/// Taken from https://github.com/Rust-SDL2/rust-sdl2#readme and
/// modified to use an `anyhow::Result`.
fn find_sdl_gl_driver() -> Result<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Ok(index as u32);
        }
    }
    Err(anyhow!("SDL OpenGL driver not found!"))
}
