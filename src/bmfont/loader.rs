// bmfont/loader.rs
use anyhow::{Result};
use std::collections::HashMap;

use crate::bmfont::types::BmFontChar;

pub struct BmFontRaw {
    pub line_height: u16,
    pub base: u16,
    pub scale_w: u16,
    pub scale_h: u16,
    pub page_file: String,
    pub chars: HashMap<u32, BmFontChar>,
}


pub struct BmFontLoader;

impl BmFontLoader {
    pub fn parse(fnt_content: &str) -> Result<BmFontRaw> {
        let mut chars = HashMap::new();
        let mut line_height = 20;
        let mut base = 16;
        let mut scale_w = 130;
        let mut scale_h = 132;
        let mut page_file = String::new();

        for line in fnt_content.lines() {
            if line.starts_with("common") {
                for part in line.split_whitespace() {
                    if part.starts_with("lineHeight=") {
                        line_height = part[11..].parse()?;
                    } else if part.starts_with("base=") {
                        base = part[5..].parse()?;
                    } else if part.starts_with("scaleW=") {
                        scale_w = part[7..].parse()?;
                    } else if part.starts_with("scaleH=") {
                        scale_h = part[7..].parse()?;
                    }
                }
            } else if line.starts_with("page") {
                for part in line.split_whitespace() {
                    if part.starts_with("file=") {
                        page_file = part[5..].trim_matches('"').to_string();
                    }
                }
            }else if line.starts_with("char") {
                let mut id = 0;
                let mut x = 0;
                let mut y = 0;
                let mut width = 0;
                let mut height = 0;
                let mut xoffset = 0;
                let mut yoffset = 0;
                let mut xadvance = 0;

                for part in line.split_whitespace() {
                    let Some(eq_pos) = part.find('=') else { continue };
                    let key = &part[..eq_pos];
                    let value = &part[eq_pos+1..];
                    match key {
                        "id" => id = value.parse().unwrap_or(0),
                        "x" => x = value.parse().unwrap_or(0),
                        "y" => y = value.parse().unwrap_or(0),
                        "width" => width = value.parse().unwrap_or(0),
                        "height" => height = value.parse().unwrap_or(0),
                        "xoffset" => xoffset = value.parse().unwrap_or(0),
                        "yoffset" => yoffset = value.parse().unwrap_or(0),
                        "xadvance" => xadvance = value.parse().unwrap_or(0),
                        _ => {}
                    }
                }
                chars.insert(id, BmFontChar {
                    id, x, y, width, height, xoffset, yoffset, xadvance,
                });
            }
        }


        Ok(BmFontRaw {
            line_height,
            base,
            scale_w,
            scale_h,
            page_file,
            chars,
        })
    }
}
