use wgpu_simple_ui::common::types::{FontLoader, RawGlyph};

use crate::bmfont::font_manager::FontManager;
use std::io::Cursor;

pub struct BmFontLoaderAdapter {
    manager: FontManager,
}

impl BmFontLoaderAdapter {
    pub fn new() -> Self {
        Self {
            manager: FontManager::new(),
        }
    }
}

impl FontLoader for BmFontLoaderAdapter {
    fn load_font_data(&self, name: &str) -> Option<(Vec<u8>, u32, u32, Vec<RawGlyph>)> {
        let data = self.manager.get_font_data(name)?;

        // Декодируем PNG в RGBA через ImageReader
        let img = image::ImageReader::new(Cursor::new(&data.png_data))
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());

        // Преобразуем BmFontChar в RawGlyph
        let glyphs: Vec<RawGlyph> = data
            .raw
            .chars
            .iter()
            .map(|(&id, ch)| RawGlyph {
                id,
                width: ch.width,
                height: ch.height,
                xoffset: ch.xoffset,
                yoffset: ch.yoffset,
                xadvance: ch.xadvance,
                x: ch.x,
                y: ch.y,
            })
            .collect();

        Some((rgba.into_raw(), w, h, glyphs))
    }
}