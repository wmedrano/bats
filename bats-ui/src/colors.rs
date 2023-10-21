use sdl2::pixels::Color;

/// Holds colors to form a cohesive color scheme.
#[derive(Copy, Clone, Debug)]
pub struct ColorScheme {
    /// The foreground color.
    pub foreground: Color,
    /// The background color.
    pub background: Color,
}

impl Default for ColorScheme {
    fn default() -> ColorScheme {
        ColorScheme {
            foreground: Color::RGB(255, 255, 255),
            background: Color::RGB(0, 0, 0),
        }
    }
}
