use wgpu_simple_ui::common::types::TextureLoader;
use std::path::PathBuf;

pub struct FileTextureLoader;

impl TextureLoader for FileTextureLoader {
    fn load_texture_rgba(&self, path: &str) -> Option<(Vec<u8>, u32, u32)> {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let full_path = PathBuf::from(manifest_dir).join(path);


        let img = image::open(full_path).ok()?;
        let rgba = img.to_rgba8();
        let width =  rgba.width();
        let height = rgba.height();
        Some((rgba.into_raw(), width, height))
    }
}