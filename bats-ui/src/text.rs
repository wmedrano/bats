use std::{collections::HashMap, sync::OnceLock};

use anyhow::Result;
use sdl2::{
    pixels::Color,
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    ttf::{Font, FontStyle},
    video::{Window, WindowContext},
};

/// Renders text onto an sdl2 canvas.
pub struct TextRenderer {
    font: Font<'static, 'static>,
    texture_creator: TextureCreator<WindowContext>,
    texture_cache: HashMap<CacheKey, CachedTexture>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    // TODO: Allow a `Cow<'a, str>` as the text.
    text: String,
    style: FontStyle,
    color: Color,
}

struct CachedTexture {
    texture: Texture<'static>,
    width: u32,
    height: u32,
    accesses: usize,
}

impl TextRenderer {
    /// Create a new `TextRenderer`.
    pub fn new(c: &Canvas<Window>) -> Result<TextRenderer> {
        let font = ttf_context()
            .load_font("assets/FiraMono-Medium.ttf", 16)
            .map_err(anyhow::Error::msg)?;
        let texture_creator = c.texture_creator();
        Ok(TextRenderer {
            font,
            texture_creator,
            texture_cache: HashMap::new(),
        })
    }

    /// Set the font style.
    pub fn set_style(&mut self, style: FontStyle) {
        self.font.set_style(style);
    }

    /// Clear the cache of any unused text that has not been seen since the last call to
    /// `clear_unused_cache`.
    pub fn clear_unused_cache(&mut self) {
        self.texture_cache.retain(|_k, v| v.accesses > 0);
    }

    /// Render a simple menu onto `dst`.
    pub fn render_menu(
        &mut self,
        dst: &mut Canvas<Window>,
        color: Color,
        header: String,
        items: impl Iterator<Item = String>,
    ) -> Result<()> {
        self.set_style(FontStyle::BOLD);
        let (_, height) = self.render(dst, header, color, (0, 0))?;
        self.set_style(FontStyle::empty());
        for (idx, item) in items.enumerate() {
            let x_y = (16, height as i32 * (idx + 1) as i32);
            self.render(dst, item, color, x_y)?;
        }
        Ok(())
    }

    /// Render text onto `dst`.
    pub fn render(
        &mut self,
        dst: &mut Canvas<Window>,
        text: String,
        color: Color,
        x_y: (i32, i32),
    ) -> Result<(u32, u32)> {
        if text.is_empty() {
            return Ok((0, 0));
        }
        let key = CacheKey {
            text,
            style: self.font.get_style(),
            color: dst.draw_color(),
        };
        if !self.texture_cache.contains_key(&key) {
            let surface = self.font.render(&key.text).blended(color).unwrap();
            let (width, height) = (surface.width(), surface.height());
            let texture = self
                .texture_creator
                .create_texture_from_surface(&surface)
                .unwrap();
            // We transmute to extend the lifetime. This is fine as the lifetime of texture and
            // texture_creator are both linked to `self`.
            let texture: Texture<'static> = unsafe { std::mem::transmute(texture) };
            self.texture_cache.insert(
                key.clone(),
                CachedTexture {
                    texture,
                    width,
                    height,
                    accesses: 0,
                },
            );
        }

        let cached_texture = self.texture_cache.get_mut(&key).unwrap();
        cached_texture.accesses += 1;
        Self::render_texture(
            dst,
            &cached_texture.texture,
            x_y.0,
            x_y.1,
            cached_texture.width,
            cached_texture.height,
        )?;
        Ok((cached_texture.width, cached_texture.height))
    }

    /// Render text onto `dst`.
    fn render_texture(
        dst: &mut Canvas<Window>,
        texture: &Texture,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        dst.copy(
            texture,
            Rect::new(0, 0, width, height),
            Rect::new(x, y, width, height),
        )
        .unwrap();
        Ok(())
    }
}

fn ttf_context() -> &'static sdl2::ttf::Sdl2TtfContext {
    static MEM: OnceLock<sdl2::ttf::Sdl2TtfContext> = OnceLock::new();
    MEM.get_or_init(|| sdl2::ttf::init().unwrap())
}
