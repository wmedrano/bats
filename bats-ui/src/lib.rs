use anyhow::{anyhow, Result};
use colors::ColorScheme;
use log::info;
use sdl2::{
    event::Event, keyboard::Keycode, render::Canvas, ttf::FontStyle, video::Window, EventPump,
};
use text::TextRenderer;

pub mod colors;
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

/// Runs the Ui.
pub struct Ui {
    /// The canvas to render items onto.
    canvas: Canvas<Window>,
    /// An iterator over events.
    event_pump: EventPump,
    /// The color scheme to use.
    color_scheme: ColorScheme,
    /// The text renderer.
    text_renderer: TextRenderer,
    /// The name of the current plugins.
    plugin_names: Vec<&'static str>,
    /// The frame number.
    frame_number: usize,
}

impl Ui {
    /// Create a new `Ui`.
    pub fn new(plugin_names: Vec<&'static str>) -> Result<Ui> {
        let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
        let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
        let window = video_subsystem
            .window("bats", 320, 240)
            .opengl()
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
        info!("UI initialized.");
        Ok(Ui {
            canvas,
            event_pump: event_iter,
            color_scheme,
            text_renderer,
            plugin_names,
            frame_number: 0,
        })
    }

    /// Run the UI. This function will keep running until the user
    /// requests an exit.
    pub fn run(&mut self) -> Result<()> {
        while self.handle_events() == ProgramRequest::Continue {
            self.frame_number += 1;
            self.render(self.frame_number);
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
                    keycode: Some(Keycode::Escape),
                    ..
                } => return ProgramRequest::Exit,
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

        self.text_renderer.set_style(FontStyle::BOLD);
        let (_, height) = self
            .text_renderer
            .render(
                &mut self.canvas,
                "active plugins".to_string(),
                self.color_scheme.foreground,
                (0, 0),
            )
            .unwrap();
        self.text_renderer.set_style(FontStyle::empty());
        for (idx, plugin_name) in self.plugin_names.iter().enumerate() {
            let y = (idx + 1) as i32 * height as i32;
            self.text_renderer
                .render(
                    &mut self.canvas,
                    plugin_name.to_string(),
                    self.color_scheme.foreground,
                    (16, y),
                )
                .unwrap();
        }
        if frame_number % 256 == 0 {
            self.text_renderer.clear_unused_cache();
        }

        self.canvas.present();
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
