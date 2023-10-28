use anyhow::{anyhow, Result};
use bats_async::CommandSender;
use bats_lib::{
    plugin::{toof::Toof, BatsInstrument},
    Bats,
};
use bats_state::{BatsState, TrackDetails};
use colors::ColorScheme;
use frame_counter::FrameCounter;
use log::{info, warn};
use sdl2::{event::Event, keyboard::Keycode, render::Canvas, video::Window, EventPump};
use text::TextRenderer;

pub mod bats_state;
pub mod colors;
pub mod frame_counter;
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
    /// The metronome.
    Metronome,
}

/// Runs the Ui.
pub struct Ui {
    /// The current page.
    page: Page,
    /// The state for the track menu.
    tracks_menu: TracksMenuState,
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

impl Ui {
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
            tracks_menu: TracksMenuState { selected_idx: 0 },
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
                        .move_selection(-1, self.bats_state.tracks()),
                    (Page::TracksMenu, Keycode::Down) => {
                        self.tracks_menu.move_selection(1, self.bats_state.tracks())
                    }
                    (Page::TracksMenu, Keycode::Return) => {
                        match self.tracks_menu.selection(self.bats_state.tracks()) {
                            Some(p) => self.bats_state.set_armed(Some(p.id)),
                            None => {
                                let id = self
                                    .bats_state
                                    .add_plugin(Toof::new(self.bats_state.sample_rate))
                                    .id;
                                self.bats_state.set_armed(Some(id));
                            }
                        }
                    }
                    _ => (),
                },
                Event::TextInput { text, .. } => match (self.page, text.as_str()) {
                    (_, "M") => self.bats_state.toggle_metronome(),
                    (Page::MainMenu, "m") => self.page = Page::Metronome,
                    (Page::MainMenu, "t") => self.page = Page::TracksMenu,
                    (Page::MainMenu, "q") => return ProgramRequest::Exit,
                    (Page::Metronome, "+") => self.bats_state.set_bpm(self.bats_state.bpm() + 0.5),
                    (Page::Metronome, "-") => self.bats_state.set_bpm(self.bats_state.bpm() - 0.5),
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
        let items = std::iter::once("Add Track".to_string())
            .chain(self.bats_state.tracks().map(|s| s.name.to_string()));
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

/// Contains the plugins menu state.
#[derive(Copy, Clone, Debug)]
struct TracksMenuState {
    /// The selected index. Note that "0" is reserved for add.
    selected_idx: usize,
}

impl TracksMenuState {
    /// Get the current selection or `None` if "add track" is selected.
    fn selection<'a>(
        &self,
        tracks: impl Iterator<Item = &'a TrackDetails>,
    ) -> Option<TrackDetails> {
        let mut tracks = tracks;
        if self.selected_idx == 0 {
            None
        } else {
            tracks.nth(self.selected_idx - 1).cloned()
        }
    }

    /// Move the selection by `n`. If `n` is negative, the selection will move backwards.
    fn move_selection<'a>(&mut self, n: isize, tracks: impl Iterator<Item = &'a TrackDetails>) {
        let n_choices = tracks.count() as isize + 1;
        let idx = (self.selected_idx as isize + n) % n_choices;
        if idx < 0 {
            self.selected_idx = (idx + n_choices) as usize;
        } else {
            self.selected_idx = idx as usize;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK_DETAIL: TrackDetails = TrackDetails {
        id: 0,
        name: "test plugin",
    };

    fn fake_track_details(cnt: usize) -> impl Iterator<Item = &'static TrackDetails> {
        std::iter::repeat(&TRACK_DETAIL).take(cnt)
    }

    #[test]
    fn move_selection_advances_selection() {
        let mut state = TracksMenuState { selected_idx: 1 };
        state.move_selection(2, fake_track_details(100));
        assert_eq!(state.selected_idx, 3);
    }

    #[test]
    fn move_selection_wraps_around() {
        let mut state = TracksMenuState { selected_idx: 2 };
        state.move_selection(3, fake_track_details(4));
        assert_eq!(state.selected_idx, 0);
    }

    #[test]
    fn move_selection_addvances_selection_backward() {
        let mut state = TracksMenuState { selected_idx: 2 };
        state.move_selection(-1, fake_track_details(100));
        assert_eq!(state.selected_idx, 1);
    }

    #[test]
    fn move_selection_wraps_around_backwards() {
        let mut state = TracksMenuState { selected_idx: 2 };
        state.move_selection(-3, fake_track_details(4));
        assert_eq!(state.selected_idx, 4);
    }
}
