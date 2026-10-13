use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::Path;
use crate::capture::CaptureMode;
use xcap::{Monitor, Window};

pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || capture_blocking(mode))
        .await
        .context("Failed to join Windows capture task")?
}

fn capture_blocking(mode: CaptureMode) -> Result<RgbaImage> {
    tracing::info!("Triggering Windows screen capture (mode: {:?})", mode);
    match mode {
        CaptureMode::Fullscreen | CaptureMode::Region => capture_monitors(),
        CaptureMode::Window => capture_active_window(),
    }
}

fn capture_monitors() -> Result<RgbaImage> {
    let monitors = Monitor::all().context("Failed to enumerate monitors on Windows")?;
    if monitors.is_empty() {
        anyhow::bail!("No monitors detected on Windows system");
    }

    if monitors.len() == 1 {
        return monitors[0].capture_image().context("Failed to capture primary monitor");
    }

    // Multi-monitor: calculate bounding box spanning all displays
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for m in &monitors {
        let x = m.x().unwrap_or(0);
        let y = m.y().unwrap_or(0);
        let w = m.width().unwrap_or(1920) as i32;
        let h = m.height().unwrap_or(1080) as i32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let total_w = (max_x - min_x).max(1) as u32;
    let total_h = (max_y - min_y).max(1) as u32;
    let mut combined = RgbaImage::new(total_w, total_h);

    for m in &monitors {
        if let Ok(img) = m.capture_image() {
            let offset_x = (m.x().unwrap_or(0) - min_x).max(0) as i64;
            let offset_y = (m.y().unwrap_or(0) - min_y).max(0) as i64;
            image::imageops::overlay(&mut combined, &img, offset_x, offset_y);
        }
    }

    Ok(combined)
}

fn capture_active_window() -> Result<RgbaImage> {
    let windows = Window::all().context("Failed to enumerate windows on Windows")?;
    
    // Find the first non-minimized foreground application window that is not BenderShot itself
    for w in windows {
        let is_min = w.is_minimized().unwrap_or(false);
        if !is_min {
            let app_name = w.app_name().unwrap_or_default().to_lowercase();
            let title = w.title().unwrap_or_default().to_lowercase();
            if !app_name.contains("bendershot") && !title.contains("bendershot") {
                if let Ok(img) = w.capture_image() {
                    return Ok(img);
                }
            }
        }
    }

    // Fallback to monitor capture
    capture_monitors()
}

pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
    let dynamic = image::open(path)
        .with_context(|| format!("Failed to open image at {:?}", path))?;
    Ok(dynamic.to_rgba8())
}
