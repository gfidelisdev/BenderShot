use anyhow::Result;
use image::RgbaImage;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Region,
    Fullscreen,
    Window,
}

/// Region chosen on the live-screen overlay, in the overlay's logical points.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RegionSelection {
    /// `[min_x, min_y, max_x, max_y]` relative to the overlay's top-left corner.
    pub rect: [f32; 4],
    /// Overlay size in logical points (either full-desktop layout size or single-monitor size).
    pub overlay_size: [f32; 2],
}

/// One logical monitor of the desktop layout.
/// `x, y, w, h` are in compositor layout coordinates; `lw, lh` are the logical size
/// (what a fullscreen overlay window on that monitor measures in points).
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct MonitorLayout {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub lw: f32,
    pub lh: f32,
    #[serde(default)]
    pub primary: bool,
}

/// Picks the monitor the overlay was shown on (or full desktop if the overlay spans all monitors)
/// and crops the selection out of a screenshot that spans the whole desktop layout.
pub fn crop_selection_from_desktop(
    desktop: &RgbaImage,
    monitors: &[MonitorLayout],
    sel: &RegionSelection,
) -> Option<RgbaImage> {
    let [ow, oh] = sel.overlay_size;
    let fallback_single = MonitorLayout {
        x: 0.0,
        y: 0.0,
        w: ow,
        h: oh,
        lw: ow,
        lh: oh,
        primary: true,
    };
    let monitors: Vec<MonitorLayout> = if monitors.is_empty() { vec![fallback_single] } else { monitors.to_vec() };

    let min_x = monitors.iter().map(|m| m.x).fold(f32::MAX, f32::min);
    let min_y = monitors.iter().map(|m| m.y).fold(f32::MAX, f32::min);
    let max_x = monitors.iter().map(|m| m.x + m.w).fold(f32::MIN, f32::max);
    let max_y = monitors.iter().map(|m| m.y + m.h).fold(f32::MIN, f32::max);
    let layout_w = (max_x - min_x).max(1.0);
    let layout_h = (max_y - min_y).max(1.0);

    let (x0, y0, x1, y1) = if (ow - layout_w).abs() <= 4.0 && (oh - layout_h).abs() <= 4.0 {
        // Overlay spans the entire desktop layout directly (e.g. X11 multi-monitor RGBA overlay).
        let kx = desktop.width() as f32 / ow.max(1.0);
        let ky = desktop.height() as f32 / oh.max(1.0);
        (
            (sel.rect[0] * kx).round().clamp(0.0, desktop.width() as f32) as u32,
            (sel.rect[1] * ky).round().clamp(0.0, desktop.height() as f32) as u32,
            (sel.rect[2] * kx).round().clamp(0.0, desktop.width() as f32) as u32,
            (sel.rect[3] * ky).round().clamp(0.0, desktop.height() as f32) as u32,
        )
    } else {
        let matches: Vec<&MonitorLayout> = monitors
            .iter()
            .filter(|m| (m.lw - ow).abs() <= 4.0 && (m.lh - oh).abs() <= 4.0)
            .collect();
        let mon = match matches.as_slice() {
            [one] => **one,
            [] | [_, ..] => {
                *matches
                    .iter()
                    .find(|m| m.primary)
                    .copied()
                    .or_else(|| monitors.iter().find(|m| m.primary))
                    .unwrap_or(&monitors[0])
            }
        };

        let kx = desktop.width() as f32 / layout_w;
        let ky = desktop.height() as f32 / layout_h;
        let fx = mon.w / mon.lw.max(1.0);
        let fy = mon.h / mon.lh.max(1.0);

        let to_px = |v: f32, origin: f32, f: f32, layout_min: f32, k: f32| ((origin + v * f - layout_min) * k).round();
        (
            to_px(sel.rect[0], mon.x, fx, min_x, kx).clamp(0.0, desktop.width() as f32) as u32,
            to_px(sel.rect[1], mon.y, fy, min_y, ky).clamp(0.0, desktop.height() as f32) as u32,
            to_px(sel.rect[2], mon.x, fx, min_x, kx).clamp(0.0, desktop.width() as f32) as u32,
            to_px(sel.rect[3], mon.y, fy, min_y, ky).clamp(0.0, desktop.height() as f32) as u32,
        )
    };

    if x1 <= x0 + 1 || y1 <= y0 + 1 {
        return None;
    }
    Some(image::imageops::crop_imm(desktop, x0, y0, x1 - x0, y1 - y0).to_image())
}

/// Crops away fully/mostly transparent borders (e.g. window stream buffers larger than the
/// window, or translucent client-side shadows). Returns the input unchanged if nothing is opaque.
pub fn trim_transparent(img: RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] >= 250 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 == u32::MAX || (x0 == 0 && y0 == 0 && x1 + 1 == w && y1 + 1 == h) {
        return img;
    }
    image::imageops::crop_imm(&img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image()
}

pub struct CaptureBackend;

impl CaptureBackend {
    pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
        crate::platform::imp::capture::capture(mode).await
    }

    /// Captures the desktop *after* the selection overlay has closed and crops the selection.
    pub async fn capture_region(sel: RegionSelection) -> Result<RgbaImage> {
        crate::platform::imp::capture::capture_region(sel).await
    }

    pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
        crate::platform::imp::capture::load_image_from_path(path)
    }
}
