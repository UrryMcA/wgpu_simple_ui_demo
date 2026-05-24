// bmfont/types.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BmFontChar {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub xoffset: i16,
    pub yoffset: i16,
    pub xadvance: i16,
}

#[derive(Debug, Clone)]
pub struct BmFont {
    pub texture_id: u64,
    pub line_height: u16,
    pub base: u16,
    pub scale_w: u16,
    pub scale_h: u16,
    pub chars: HashMap<u32, BmFontChar>,
}
