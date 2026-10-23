use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::Path;
use crate::capture::{CaptureMode, MonitorLayout, RegionSelection};
use xcap::Monitor;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    GetWindowDC, IntersectRect, MonitorFromWindow, ReleaseDC, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    SRCCOPY,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClassNameW, GetDesktopWindow, GetForegroundWindow,
    GetLayeredWindowAttributes, GetShellWindow, GetSystemMetrics, GetTopWindow, GetWindow,
    GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, InternalGetWindowText, IsIconic,
    IsWindow, IsWindowVisible, IsZoomed, SetForegroundWindow, SetProcessDPIAware, ShowWindow,
    GA_ROOT, GA_ROOTOWNER, GWL_EXSTYLE, GW_HWNDNEXT, GW_HWNDPREV, LWA_ALPHA, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOWNOACTIVATE,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

const PW_RENDERFULLCONTENT: u32 = 0x00000002;

pub fn ensure_dpi_aware() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            SetProcessDPIAware();
        }
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

/// Checks if `pid` belongs to any running instance of `bendershot.exe` by inspecting
/// the kernel process image name rather than the window title (so windows like VS Code
/// or Explorer with "BenderShot" in their title are never falsely skipped).
unsafe fn is_bendershot_process(pid: u32, current_pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if pid == current_pid {
        return true;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 as _ {
            return false;
        }
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok != 0 && size > 0 {
            let path_str = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
            if path_str.ends_with("\\bendershot.exe") || path_str.ends_with("/bendershot.exe") {
                return true;
            }
            if let Ok(cur_exe) = std::env::current_exe() {
                if let Some(cur_name) = cur_exe.file_name().and_then(|n| n.to_str()) {
                    let cur_lower = cur_name.to_lowercase();
                    if path_str.ends_with(&format!("\\{}", cur_lower))
                        || path_str.ends_with(&format!("/{}", cur_lower))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
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

        // When a window is maximized on Windows 10/11, its frame bounds extend ~8px outside
        // the monitor edges. Clamp to the monitor's rect so we don't capture adjacent monitors
        // or out-of-bounds black borders.
        if IsZoomed(hwnd) != 0 {
            let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if hmon != 0 as _ {
                let mut minfo: MONITORINFO = std::mem::zeroed();
                minfo.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(hmon, &mut minfo) != 0 {
                    rect.left = rect.left.max(minfo.rcMonitor.left);
                    rect.top = rect.top.max(minfo.rcMonitor.top);
                    rect.right = rect.right.min(minfo.rcMonitor.right);
                    rect.bottom = rect.bottom.min(minfo.rcMonitor.bottom);
                }
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
    require_title: bool,
) -> bool {
    unsafe {
        if hwnd == 0 as _ || hwnd == desktop_hwnd || hwnd == shell_hwnd {
            return false;
        }
        if IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if is_bendershot_process(pid, current_pid) {
            return false;
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let is_app_window = (ex_style & WS_EX_APPWINDOW) != 0;
        if !is_app_window
            && ((ex_style & WS_EX_TOOLWINDOW) != 0 || (ex_style & WS_EX_NOACTIVATE) != 0)
        {
            return false;
        }

        // Skip click-through transparent overlays (e.g. GPU/gaming/screen overlays)
        if (ex_style & WS_EX_TRANSPARENT) != 0 && (ex_style & WS_EX_LAYERED) != 0 {
            return false;
        }

        // Skip invisible layered windows (alpha ~ 0)
        if (ex_style & WS_EX_LAYERED) != 0 {
            let mut cr = 0u32;
            let mut alpha = 255u8;
            let mut flags = 0u32;
            if GetLayeredWindowAttributes(hwnd, &mut cr, &mut alpha, &mut flags) != 0 {
                if (flags & LWA_ALPHA) != 0 && alpha < 10 {
                    return false;
                }
            }
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
                    | "BenderShotCountdownHUD"
                    | "Shell_TrayWnd"
                    | "Shell_SecondaryTrayWnd"
                    | "NotifyIconOverflowWindow"
                    | "TopLevelWindowForOverflowXamlIsland"
                    | "Progman"
                    | "WorkerW"
                    | "Windows.UI.Core.CoreWindow"
                    | "XamlExplorerHostIslandWindow"
                    | "CEF-OSC-WIDGET"
                    | "IME"
                    | "MSCTFIME UI"
                    | "DummyDWMListenerWindow"
                    | "EdgeUiInputTopWndClass"
                    | "EdgeUiInputWndClass"
            ) {
                return false;
            }
        }

        if require_title {
            let mut title_buf = [0u16; 256];
            let title_len =
                InternalGetWindowText(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
            if title_len <= 0 {
                return false;
            }
            let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
            if title.trim().is_empty() {
                return false;
            }
        }

        match get_window_visible_rect(hwnd) {
            Some(r) => (r.right - r.left) > 50 && (r.bottom - r.top) > 50,
            None => false,
        }
    }
}

pub unsafe fn find_target_foreground_window() -> Option<HWND> {
    unsafe {
        ensure_dpi_aware();
        let current_pid = GetCurrentProcessId();
        let desktop_hwnd = GetDesktopWindow();
        let shell_hwnd = GetShellWindow();

        // 1. Check if the daemon locked in the exact target HWND at the instant the shortcut/tray was triggered
        if let Ok(val) = std::env::var("BENDERSHOT_TARGET_HWND") {
            if let Ok(raw) = val.trim().parse::<usize>() {
                if raw != 0 {
                    let candidate = raw as HWND;
                    let root = GetAncestor(candidate, GA_ROOT);
                    let target = if root != 0 as _ { root } else { candidate };
                    if is_capturable_app_window(
                        target,
                        current_pid,
                        desktop_hwnd,
                        shell_hwnd,
                        false,
                    ) {
                        return Some(target);
                    }
                }
            }
        }

        // 2. Check current foreground window (with brief retry in case Alt key / focus transition temporarily yields 0)
        for attempt in 0..5 {
            let fg = GetForegroundWindow();
            if fg != 0 as _ {
                let root = GetAncestor(fg, GA_ROOT);
                let target = if root != 0 as _ { root } else { fg };
                if is_capturable_app_window(target, current_pid, desktop_hwnd, shell_hwnd, false) {
                    return Some(target);
                }
                break;
            }
            if attempt < 4 {
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }

        // 3. Walk Z-order from top to bottom to find the topmost visible titled user application window
        let mut next = GetTopWindow(0 as _);
        while next != 0 as _ {
            if is_capturable_app_window(next, current_pid, desktop_hwnd, shell_hwnd, true) {
                return Some(next);
            }
            next = GetWindow(next, GW_HWNDNEXT);
        }
        None
    }
}

/// Checks whether `target_hwnd` is completely within the virtual screen and not occluded
/// by any other visible application window above it in Z-order.
unsafe fn is_window_unobscured_on_screen(target_hwnd: HWND, vis_rect: &RECT) -> bool {
    unsafe {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if vw > 0 && vh > 0 {
            if vis_rect.left < vx - 2
                || vis_rect.top < vy - 2
                || vis_rect.right > vx + vw + 2
                || vis_rect.bottom > vy + vh + 2
            {
                return false;
            }
        }

        let current_pid = GetCurrentProcessId();
        let desktop_hwnd = GetDesktopWindow();
        let shell_hwnd = GetShellWindow();

        let mut prev = GetWindow(target_hwnd, GW_HWNDPREV);
        while prev != 0 as _ {
            // Owned popups/dialogs belonging to target_hwnd itself are part of the target app
            let root_owner = GetAncestor(prev, GA_ROOTOWNER);
            if root_owner != target_hwnd
                && is_capturable_app_window(prev, current_pid, desktop_hwnd, shell_hwnd, true)
            {
                if let Some(other_rect) = get_window_visible_rect(prev) {
                    let mut overlap: RECT = std::mem::zeroed();
                    if IntersectRect(&mut overlap, vis_rect, &other_rect) != 0 {
                        return false;
                    }
                }
            }
            prev = GetWindow(prev, GW_HWNDPREV);
        }
        true
    }
}

/// Temporarily hides any visible `bendershot.exe` windows (such as an existing Editor window)
/// so they never occlude the target window during capture.
unsafe fn hide_visible_bendershot_windows() -> Vec<HWND> {
    let mut hidden = Vec::new();
    unsafe {
        let current_pid = GetCurrentProcessId();
        let mut next = GetTopWindow(0 as _);
        while next != 0 as _ {
            if IsWindowVisible(next) != 0 && IsIconic(next) == 0 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(next, &mut pid);
                if is_bendershot_process(pid, current_pid) {
                    let mut class_buf = [0u16; 128];
                    let class_len =
                        GetClassNameW(next, class_buf.as_mut_ptr(), class_buf.len() as i32);
                    let class_name = if class_len > 0 {
                        String::from_utf16_lossy(&class_buf[..class_len as usize])
                    } else {
                        String::new()
                    };
                    if class_name != "BenderShotTrayClass" {
                        ShowWindow(next, SW_HIDE);
                        hidden.push(next);
                    }
                }
            }
            next = GetWindow(next, GW_HWNDNEXT);
        }
    }
    hidden
}

unsafe fn restore_bendershot_windows(hidden: &[HWND]) {
    unsafe {
        for &h in hidden {
            if IsWindow(h) != 0 {
                ShowWindow(h, SW_SHOWNOACTIVATE);
            }
        }
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
            let mut non_zero_pixels = 0usize;
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
                    non_zero_pixels += 1;
                }
            }
            let total_pixels = (crop_w as usize) * (crop_h as usize);
            // Require at least 5% non-black pixels so a failed black client area is rejected
            if total_pixels > 0 && non_zero_pixels * 20 >= total_pixels {
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
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let (left, top, right, bottom) = if vw > 0 && vh > 0 {
            (
                rect.left.max(vx),
                rect.top.max(vy),
                rect.right.min(vx + vw),
                rect.bottom.min(vy + vh),
            )
        } else {
            (rect.left, rect.top, rect.right, rect.bottom)
        };

        let w = right - left;
        let h = bottom - top;
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
        let ok = BitBlt(
            mem_dc,
            0,
            0,
            w,
            h,
            screen_dc,
            left,
            top,
            SRCCOPY | CAPTUREBLT,
        );

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
            let hidden_bs = hide_visible_bendershot_windows();
            let fg = GetForegroundWindow();
            let root_fg = if fg != 0 as _ {
                let r = GetAncestor(fg, GA_ROOT);
                if r != 0 as _ { r } else { fg }
            } else {
                0 as _
            };

            if root_fg != target_hwnd {
                SetForegroundWindow(target_hwnd);
                std::thread::sleep(std::time::Duration::from_millis(60));
            } else if !hidden_bs.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(60));
            }

            let captured = if let Some(vis_rect) = get_window_visible_rect(target_hwnd) {
                if is_window_unobscured_on_screen(target_hwnd, &vis_rect) {
                    // Window is completely visible on top of the screen: capture live DWM pixels first
                    capture_screen_rect(&vis_rect)
                        .or_else(|| capture_hwnd_print_window(target_hwnd, &vis_rect))
                } else {
                    // Window is partially off-screen or behind an always-on-top window: try PrintWindow first
                    capture_hwnd_print_window(target_hwnd, &vis_rect)
                        .or_else(|| capture_screen_rect(&vis_rect))
                }
            } else {
                None
            };

            restore_bendershot_windows(&hidden_bs);

            if let Some(img) = captured {
                return Ok(img);
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
