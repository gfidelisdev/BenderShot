use anyhow::Result;
use image::RgbaImage;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Region,
    Fullscreen,
    Window,
}

pub struct CaptureBackend;

impl CaptureBackend {
    pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
        crate::platform::imp::capture::capture(mode).await
    }

    pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
        crate::platform::imp::capture::load_image_from_path(path)
    }
}
