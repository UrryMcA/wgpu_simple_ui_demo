use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::Deserialize;
use super::loader::BmFontLoader;

pub struct FontManager {
    pub fonts_data: HashMap<String, LoadedFontData>,
}

pub struct LoadedFontData {
    pub png_data: Vec<u8>,
    pub raw: super::loader::BmFontRaw,
}

impl FontManager {
    /// Создаёт менеджер, загружает конфиг (путь из env или по умолчанию), резолвит путь.
    pub fn new() -> Self {
        let config_path_str = std::env::var("FONTS_CONFIG_PATH")
            .unwrap_or_else(|_| "assets/fonts/fonts.json".to_string());
        // Резолвим путь к конфигу
        let config_path = match Self::resolve_path(Path::new("."), &config_path_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to resolve font config path: {}", e);
                PathBuf::from(&config_path_str) // fallback
            }
        };
        let fonts_data = Self::load_fonts_from_config(&config_path).unwrap_or_else(|e| {
            eprintln!("Failed to load fonts from {}: {}", config_path.display(), e);
            HashMap::new()
        });
        Self { fonts_data }
    }

    fn load_fonts_from_config(config_path: &Path) -> Result<HashMap<String, LoadedFontData>> {
        let config_content = fs::read_to_string(config_path)
            .with_context(|| format!("Cannot read config file: {}", config_path.display()))?;
        #[derive(Deserialize)]
        struct FontEntry { name: String, fnt: String, png: String }
        #[derive(Deserialize)]
        struct FontsConfig { fonts: Vec<FontEntry> }
        let config: FontsConfig = serde_json::from_str(&config_content)?;
        let config_dir = config_path.parent().unwrap_or(Path::new("."));
        let mut map = HashMap::new();
        for entry in config.fonts {
            let fnt_path = Self::resolve_path(config_dir, &entry.fnt)?;
            let png_path = Self::resolve_path(config_dir, &entry.png)?;
            let png_data = fs::read(&png_path)
                .with_context(|| format!("Failed to read PNG: {}", png_path.display()))?;
            let fnt_content = fs::read_to_string(&fnt_path)
                .with_context(|| format!("Failed to read FNT: {}", fnt_path.display()))?;
            let raw = BmFontLoader::parse(&fnt_content)?;
            map.insert(entry.name, LoadedFontData { png_data, raw });
        }
        Ok(map)
    }

    fn resolve_path(base_dir: &Path, rel_path: &str) -> Result<PathBuf> {
        let candidate = base_dir.join(rel_path);
        if candidate.exists() {
            return Ok(candidate);
        }
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
        let project_root = exe_dir.parent().and_then(|p| p.parent()).unwrap_or(exe_dir);
        let alt = project_root.join(rel_path);
        if alt.exists() {
            Ok(alt)
        } else {
            anyhow::bail!("File not found: {} (tried {:?} and {:?})", rel_path, candidate, alt);
        }
    }

    /// Возвращает итератор по загруженным шрифтам
    pub fn iter_fonts(&self) -> impl Iterator<Item = (&String, &LoadedFontData)> {
        self.fonts_data.iter()
    }

    pub fn get_font_data(&self, name: &str) -> Option<&LoadedFontData> {
        self.fonts_data.get(name)
    }
}