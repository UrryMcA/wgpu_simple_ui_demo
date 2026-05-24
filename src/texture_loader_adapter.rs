use wgpu_simple_ui::common::types::TextureLoader;

pub struct FileTextureLoader;

impl TextureLoader for FileTextureLoader {
    fn load_texture_rgba(&self, path: &str) -> Option<(Vec<u8>, u32, u32)> {
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let width =  rgba.width();
        let height = rgba.height();
        Some((rgba.into_raw(), width, height))
    }
}