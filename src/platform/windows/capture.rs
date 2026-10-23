use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::Path;
use crate::capture::{CaptureMode, MonitorLayout, RegionSelection};
use xcap::Monitor;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetWindowDC,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetTopWindow, GetWindow,
    GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, InternalGetWindowText, IsIconic,
    IsWindowVisible, SetProcessDPIAware, GWL_EXSTYLE, GW_HWNDNEXT, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

const PW_RENDERFULLCONTENT: u32 = 0x00000002;

fn ensure_dpi_aware() {
    unsafe {
        SetProcessDPIAware();
    }
}

pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || capture_blocking(mode))
        .await
        .context("Failed to join Windows capture task")?
}

/// Captures the desktop across all monitors and crops the selection using monitor layout.
pub async fn capture_region(sel: RegionSelection) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || {
        ensure_dpi_aware();
        let (desktop, layouts) = capture_monitors_with_layout()?;
        crate::capture::crop_selection_from_desktop(&desktop, &layouts, &sel)
            .context("Seleção de região vazia ou fora da tela")
    })
    .await
    .context("Failed to join Windows capture task")?
}

fn capture_blocking(mode: CaptureMode) -> Result<RgbaImage> {
    ensure_dpi_aware();
    tracing::info!("Triggering Windows screen capture (mode: {:?})", mode);
    match mode {
        CaptureMode::Fullscreen | CaptureMode::Region => capture_monitors(),
        CaptureMode::Window => capture_active_window(),
    }
}

fn layouts_from_monitors(monitors: &[Monitor]) -> Vec<MonitorLayout> {
    let mut layouts = Vec::new();
    for m in monitors {
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

pub fn get_monitor_layouts() -> Vec<MonitorLayout> {
    ensure_dpi_aware();
    let monitors = Monitor::all().unwrap_or_default();
    layouts_from_monitors(&monitors)
}

fn capture_monitors_with_layout() -> Result<(RgbaImage, Vec<MonitorLayout>)> {
    let monitors = Monitor::all().context("Failed to enumerate monitors on Windows")?;
    if monitors.is_empty() {
        anyhow::bail!("No monitors detected on Windows system");
    }

    let layouts = layouts_from_monitors(&monitors);

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

unsafe fn get_window_visible_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut rect as *mut RECT as _,
            std::mem::size_of::<RECT>() as u32,
        );
        if hr != 0 || (rect.right - rect.left) <= 0 || (rect.bottom - rect.top) <= 0 {
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return None;
            }
        }
        if (rect.right - rect.left) > 0 && (rect.bottom - rect.top) > 0 {
            Some(rect)
        } else {
            None
        }
    }
}

unsafe fn is_capturable_app_window(
    hwnd: HWND,
    current_pid: u32,
    desktop_hwnd: HWND,
    shell_hwnd: HWND,
) -> bool {
    unsafe {
        if hwnd == 0 as _ || hwnd == desktop_hwnd || hwnd == shell_hwnd {
            return false;
        }
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == current_pid {
            return false;
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let is_app_window = (ex_style & WS_EX_APPWINDOW) != 0;
        if !is_app_window
            && ((ex_style & WS_EX_TOOLWINDOW) != 0
                || (ex_style & WS_EX_NOACTIVATE) != 0
                || (ex_style & WS_EX_TRANSPARENT) != 0)
        {
            return false;
        }

        let mut cloaked: u32 = 0;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            &mut cloaked as *mut u32 as _,
            std::mem::size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return false;
        }

        let mut class_buf = [0u16; 128];
        let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
        if class_len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
            if matches!(
                class_name.as_str(),
                "BenderShotTrayClass"
                    | "Shell_TrayWnd"
                    | "Shell_SecondaryTrayWnd"
                    | "NotifyIconOverflowWindow"
                    | "TopLevelWindowForOverflowXamlIsland"
                    | "Progman"
                    | "WorkerW"
                    | "Windows.UI.Core.CoreWindow"
                    | "XamlExplorerHostIslandWindow"
            ) {
                return false;
            }
        }

        // InternalGetWindowText reads kernel window metadata directly without sending WM_GETTEXT,
        // avoiding any risk of blocking on unresponsive background processes.
        let mut title_buf = [0u16; 256];
        let title_len = InternalGetWindowText(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        if title_len > 0 {
            let title = String::from_utf16_lossy(&title_buf[..title_len as usize]).to_lowercase();
            if title.contains("bendershot") {
                return false;
            }
        }

        match get_window_visible_rect(hwnd) {
            Some(r) => (r.right - r.left) > 100 && (r.bottom - r.top) > 100,
            None => false,
        }
    }
}

unsafe fn find_target_foreground_window() -> Option<HWND> {
    unsafe {
        let current_pid = GetCurrentProcessId();
        let desktop_hwnd = GetDesktopWindow();
        let shell_hwnd = GetShellWindow();

        let fg = GetForegroundWindow();
        if is_capturable_app_window(fg, current_pid, desktop_hwnd, shell_hwnd) {
            return Some(fg);
        }

        // Walk Z-order from top to bottom to find the topmost user application window
        let mut next = GetTopWindow(0 as _);
        while next != 0 as _ {
            if is_capturable_app_window(next, current_pid, desktop_hwnd, shell_hwnd) {
                return Some(next);
            }
            next = GetWindow(next, GW_HWNDNEXT);
        }
        None
    }
}

/// Captures `hwnd` using `PrintWindow(PW_RENDERFULLCONTENT)` and crops to `vis_rect`
/// (which removes invisible Windows 10/11 DWM resize borders).
unsafe fn capture_hwnd_print_window(hwnd: HWND, vis_rect: &RECT) -> Option<RgbaImage> {
    unsafe {
        let mut win_rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut win_rect) == 0 {
            return None;
        }
        let win_w = win_rect.right - win_rect.left;
        let win_h = win_rect.bottom - win_rect.top;
        if win_w <= 0 || win_h <= 0 {
            return None;
        }

        let wnd_dc = GetWindowDC(hwnd);
        if wnd_dc == 0 as _ {
            return None;
        }
        let mem_dc = CreateCompatibleDC(wnd_dc);
        if mem_dc == 0 as _ {
            ReleaseDC(hwnd, wnd_dc);
            return None;
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = win_w;
        bmi.bmiHeader.biHeight = -win_h; // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, 0 as _, 0);
        if hbmp == 0 as _ || bits.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(hwnd, wnd_dc);
            return None;
        }

        let old_obj = SelectObject(mem_dc, hbmp);
        let ok = PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT);

        let crop_x = (vis_rect.left - win_rect.left).clamp(0, win_w - 1);
        let crop_y = (vis_rect.top - win_rect.top).clamp(0, win_h - 1);
        let crop_w = (vis_rect.right - vis_rect.left).clamp(1, win_w - crop_x);
        let crop_h = (vis_rect.bottom - vis_rect.top).clamp(1, win_h - crop_y);

        let result = if ok != 0 {
            let src = std::slice::from_raw_parts(bits as *const u8, (win_w * win_h * 4) as usize);
            let row_bytes = (crop_w as usize) * 4;
            let mut rgba = vec![0u8; (crop_w * crop_h * 4) as usize];
            let mut has_non_zero_rgb = false;
            for row in 0..crop_h {
                let src_start = ((crop_y + row) * win_w + crop_x) as usize * 4;
                let dst_start = (row as usize) * row_bytes;
                rgba[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&src[src_start..src_start + row_bytes]);
            }
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
                if (px[0] | px[1] | px[2]) != 0 {
                    has_non_zero_rgb = true;
                }
            }
            if has_non_zero_rgb {
                RgbaImage::from_raw(crop_w as u32, crop_h as u32, rgba)
            } else {
                None
            }
        } else {
            None
        };

        SelectObject(mem_dc, old_obj);
        DeleteObject(hbmp);
        DeleteDC(mem_dc);
        ReleaseDC(hwnd, wnd_dc);

        result
    }
}

/// Captures a screen rectangle directly from the desktop DC via `BitBlt`.
unsafe fn capture_screen_rect(rect: &RECT) -> Option<RgbaImage> {
    unsafe {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return None;
        }

        let screen_dc = GetDC(0 as _);
        if screen_dc == 0 as _ {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc == 0 as _ {
            ReleaseDC(0 as _, screen_dc);
            return None;
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, 0 as _, 0);
        if hbmp == 0 as _ || bits.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(0 as _, screen_dc);
            return None;
        }

        let old_obj = SelectObject(mem_dc, hbmp);
        let ok = BitBlt(mem_dc, 0, 0, w, h, screen_dc, rect.left, rect.top, SRCCOPY);

        let result = if ok != 0 {
            let src = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
            let mut rgba = src.to_vec();
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            RgbaImage::from_raw(w as u32, h as u32, rgba)
        } else {
            None
        };

        SelectObject(mem_dc, old_obj);
        DeleteObject(hbmp);
        DeleteDC(mem_dc);
        ReleaseDC(0 as _, screen_dc);

        result
    }
}

fn capture_active_window() -> Result<RgbaImage> {
    unsafe {
        if let Some(target_hwnd) = find_target_foreground_window() {
            if let Some(vis_rect) = get_window_visible_rect(target_hwnd) {
                if let Some(img) = capture_hwnd_print_window(target_hwnd, &vis_rect) {
                    return Ok(img);
                }
                if let Some(img) = capture_screen_rect(&vis_rect) {
                    return Ok(img);
                }
            }
        }
    }

    // Final fallback to monitor capture if no foreground window was found
    capture_monitors()
}

pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
    let dynamic = image::open(path)
        .with_context(|| format!("Failed to open image at {:?}", path))?;
    Ok(dynamic.to_rgba8())
}
