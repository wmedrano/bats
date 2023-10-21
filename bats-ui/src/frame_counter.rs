use std::time::Instant;

/// Utility for counting the number of frames and getting frame statistics like FPS.
pub struct FrameCounter {
    /// The current frame number.
    frames: usize,
    /// Running average of the FPS.
    fps: f64,
    /// FPS as a text string. Note: This may be updated infrequently to save on CPU.
    fps_text: String,
    /// The time of the last frame.
    last_frame: Instant,
}

impl FrameCounter {
    /// Create a new `FrameCounter`.
    pub fn new() -> FrameCounter {
        FrameCounter {
            frames: 0,
            fps: 0.0,
            fps_text: String::new(),
            last_frame: Instant::now(),
        }
    }

    /// Produce the next frame number. This also updates all internal stats.
    pub fn next_frame(&mut self) -> usize {
        let t = Instant::now();
        let duration = t.duration_since(self.last_frame);
        let new_fps = 1.0 / duration.as_secs_f64();
        let fps = 0.95 * self.fps + 0.05 * new_fps;

        self.frames = self.frames + 1;
        self.fps = fps;
        if self.frames % 100 == 0 {
            self.fps_text = format!("FPS: {:.0}", fps.round());
        }
        self.last_frame = t;

        self.frames
    }

    /// Get the FPS as a text string.
    pub fn fps(&self) -> &str {
        &self.fps_text
    }
}
