use anyhow::{Context, Result};
use ashpd::desktop::screenshot::Screenshot;
use image::RgbaImage;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Region,
    Fullscreen,
    Window,
}

pub struct CaptureBackend;

impl CaptureBackend {
    pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
        tracing::info!("Triggering Wayland screen capture via XDG Portal (mode: {:?})", mode);

        let request = Screenshot::request()
            .interactive(false)
            .modal(false);

        let response = request
            .send()
            .await
            .context("Failed to request screenshot from XDG Desktop Portal")?
            .response()
            .context("Empty response from screenshot portal")?;

        let uri = response.uri();
        let path = if let Ok(url) = url::Url::parse(uri.as_str()) {
            url.to_file_path()
                .map_err(|_| anyhow::anyhow!("Invalid file URI from portal: {}", uri))?
        } else {
            PathBuf::from(uri.as_str())
        };

        tracing::info!("Screenshot received at path: {:?}", path);
        Self::load_image_from_path(&path)
    }

    pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
        let dynamic = image::open(path)
            .with_context(|| format!("Failed to open image at {:?}", path))?;
        Ok(dynamic.to_rgba8())
    }
}
