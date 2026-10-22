use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::Path;
use crate::capture::{CaptureMode, MonitorLayout, RegionSelection};
use xcap::{Monitor, Window};

pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || capture_blocking(mode))
        .await
        .context("Failed to join Windows capture task")?
}

/// Captures the desktop across all monitors and crops the selection using monitor layout.
pub async fn capture_region(sel: RegionSelection) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || {
        let (desktop, layouts) = capture_monitors_with_layout()?;
        crate::capture::crop_selection_from_desktop(&desktop, &layouts, &sel)
            .context("Seleção de região vazia ou fora da tela")
    })
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

pub fn get_monitor_layouts() -> Vec<MonitorLayout> {
    let monitors = Monitor::all().unwrap_or_default();
    let mut layouts = Vec::new();
    for m in &monitors {
        let x = m.x().unwrap_or(0) as f32;
        let y = m.y().unwrap_or(0) as f32;
        let w = m.width().unwrap_or(1920) as f32;
        let h = m.height().unwrap_or(1080) as f32;
        let scale = m.scale_factor().unwrap_or(1.0).max(0.1);
        let lw = (w / scale).round();
        let lh = (h / scale).round();
        let primary = m.is_primary().unwrap_or(false);
        layouts.push(MonitorLayout {
            x,
            y,
            w,
            h,
            lw,
            lh,
            primary,
        });
    }
    layouts
}

fn capture_monitors_with_layout() -> Result<(RgbaImage, Vec<MonitorLayout>)> {
    let monitors = Monitor::all().context("Failed to enumerate monitors on Windows")?;
    if monitors.is_empty() {
        anyhow::bail!("No monitors detected on Windows system");
    }

    let layouts = get_monitor_layouts();

    if monitors.len() == 1 {
        let img = monitors[0].capture_image().context("Failed to capture primary monitor")?;
        return Ok((img, layouts));
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

    Ok((combined, layouts))
}

fn capture_monitors() -> Result<RgbaImage> {
    let (combined, _) = capture_monitors_with_layout()?;
    Ok(combined)
}

fn capture_active_window() -> Result<RgbaImage> {
    unsafe {
        let mut target_hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();

        // If the foreground window belongs to our own process (e.g. BenderShot daemon or helper),
        // or is null/desktop, search Z-order for the topmost non-BenderShot top-level application window.
        let current_pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
        let is_own_process = |h: windows_sys::Win32::Foundation::HWND| -> bool {
            let mut pid = 0;
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(h, &mut pid);
            pid == current_pid
        };

        let desktop_hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetDesktopWindow();
        let shell_hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetShellWindow();

        if target_hwnd == 0 as _
            || target_hwnd == desktop_hwnd
            || target_hwnd == shell_hwnd
            || is_own_process(target_hwnd)
        {
            // Iterate Z-order to find the true user foreground window
            let mut next = windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(
                desktop_hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::GW_CHILD,
            );
            while next != 0 as _ {
                if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(next) != 0
                    && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(next) == 0
                    && next != desktop_hwnd
                    && next != shell_hwnd
                    && !is_own_process(next)
                {
                    let mut cloaked: u32 = 0;
                    let _ = windows_sys::Win32::Graphics::Dwm::DwmGetWindowAttribute(
                        next,
                        windows_sys::Win32::Graphics::Dwm::DWMWA_CLOAKED as u32,
                        &mut cloaked as *mut u32 as _,
                        std::mem::size_of::<u32>() as u32,
                    );
                    if cloaked == 0 {
                        let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
                        windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(next, &mut rect);
                        if (rect.right - rect.left) > 100 && (rect.bottom - rect.top) > 100 {
                            target_hwnd = next;
                            break;
                        }
                    }
                }
                next = windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(
                    next,
                    windows_sys::Win32::UI::WindowsAndMessaging::GW_HWNDNEXT,
                );
            }
        }

        // Try to match target_hwnd in xcap's enumerated windows
        if let Ok(windows) = Window::all() {
            if target_hwnd != 0 as _ {
                if let Some(target) = windows.iter().find(|w| (w.id().unwrap_or(0) as usize) == (target_hwnd as usize)) {
                    if let Ok(img) = target.capture_image() {
                        return Ok(crate::capture::trim_transparent(img));
                    }
                }
            }

            // Fallback: search xcap windows for first valid foreground application
            for w in &windows {
                let is_min = w.is_minimized().unwrap_or(false);
                if !is_min {
                    let app_name = w.app_name().unwrap_or_default().to_lowercase();
                    let title = w.title().unwrap_or_default().to_lowercase();
                    if !app_name.contains("bendershot") && !title.contains("bendershot") {
                        if let Ok(img) = w.capture_image() {
                            return Ok(crate::capture::trim_transparent(img));
                        }
                    }
                }
            }
        }
    }

    // Final fallback to monitor capture
    capture_monitors()
}

pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
    let dynamic = image::open(path)
        .with_context(|| format!("Failed to open image at {:?}", path))?;
    Ok(dynamic.to_rgba8())
}
