use anyhow::{anyhow, Result};
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, render::Canvas, video::Window, EventPump,
};

/// Holds colors to form a cohesive color scheme.
#[derive(Copy, Clone, Debug)]
pub struct ColorScheme {
    /// The background color.
    background: Color,
}

impl Default for ColorScheme {
    fn default() -> ColorScheme {
        ColorScheme {
            background: Color::RGB(0, 0, 0),
        }
    }
}

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
}

impl Ui {
    /// Create a new `Ui`.
    pub fn new() -> Result<Ui> {
        let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
        let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
        let window = video_subsystem
            .window("bats", 640, 320)
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
        Ok(Ui {
            canvas,
            event_pump: event_iter,
            color_scheme,
        })
    }

    /// Run the UI. This function will keep running until the user
    /// requests an exit.
    pub fn run(&mut self) -> Result<()> {
        while self.handle_events() == ProgramRequest::Continue {
            self.render();
        }
        Ok(())
    }

    /// Render a new frame and present it. It should be automatically
    /// synchronized and frame limitted.
    fn render(&mut self) {
        self.canvas.set_draw_color(self.color_scheme.background);
        self.canvas.clear();
        self.canvas.present();
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
