use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use anyhow::Result;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateRoundRectRgn,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, FrameRgn,
    GetMonitorInfoW, GetStockObject, InvalidateRect, MonitorFromPoint, SelectObject, SetBkMode,
    SetTextColor, SetWindowRgn, UpdateWindow, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD,
    FW_NORMAL, MONITORINFO, MONITOR_DEFAULTTOPRIMARY, NULL_PEN, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos,
    GetSystemMetrics, PeekMessageW, PostQuitMessage, RegisterClassW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, HTTRANSPARENT, MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN,
    SW_SHOWNOACTIVATE, WM_DESTROY, WM_ERASEBKGND, WM_NCHITTEST, WM_PAINT, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

static CURRENT_COUNT: AtomicU32 = AtomicU32::new(3);
static CANCELLED_COUNTDOWN: AtomicBool = AtomicBool::new(false);
static SHORTCUT_LABEL: Mutex<String> = Mutex::new(String::new());

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-record.pid")
}

pub fn temp_video_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec.mp4")
}

pub fn video_path_file() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec-path.txt")
}

pub fn stop_flag_file() -> PathBuf {
    std::env::temp_dir().join("bendershot-stop-record.flag")
}

pub fn is_process_running(pid: u32) -> bool {
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 as _ {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        ok && code == STILL_ACTIVE
    }
}

pub fn stop_worker_process(_pid: u32) {
    let flag = stop_flag_file();
    let _ = std::fs::write(&flag, b"stop");
}

pub fn send_notification(summary: &str, body: &str, _icon: &str) {
    crate::platform::windows::shortcuts::ShortcutManager::send_notification(summary, body);
}

pub fn detect_screen_dimensions() -> (u32, u32) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN) as u32;
        if w > 0 && h > 0 {
            return (w, h);
        }
    }
    (1920, 1080)
}

unsafe extern "system" fn countdown_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => HTTRANSPARENT as LRESULT,
            WM_ERASEBKGND => 1 as LRESULT, // Prevent flickering during repaints
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                if hdc != 0 as _ {
                    let mut rc: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rc);

                    // Double-buffering: draw to memory DC then BitBlt to avoid flicker
                    let mem_dc = CreateCompatibleDC(hdc);
                    let mem_bmp = CreateCompatibleBitmap(hdc, rc.right, rc.bottom);
                    let old_bmp = SelectObject(mem_dc, mem_bmp as _);

                    // 1. Dark pill background
                    let bg_color = rgb(22, 24, 30);
                    let bg_brush = CreateSolidBrush(bg_color);
                    FillRect(mem_dc, &rc, bg_brush);
                    DeleteObject(bg_brush as _);

                    // 2. Subtle outer border
                    let border_rgn = CreateRoundRectRgn(0, 0, rc.right, rc.bottom, 22, 22);
                    let border_brush = CreateSolidBrush(rgb(75, 82, 100));
                    FrameRgn(mem_dc, border_rgn, border_brush, 2, 2);
                    DeleteObject(border_brush as _);
                    DeleteObject(border_rgn as _);

                    // 3. Red recording dot
                    let dot_brush = CreateSolidBrush(rgb(235, 52, 52));
                    let old_brush = SelectObject(mem_dc, dot_brush as _);
                    let null_pen = GetStockObject(NULL_PEN);
                    let old_pen = SelectObject(mem_dc, null_pen);
                    Ellipse(mem_dc, 20, 20, 36, 36);
                    SelectObject(mem_dc, old_pen);
                    SelectObject(mem_dc, old_brush);
                    DeleteObject(dot_brush as _);

                    // 4. Texts
                    SetBkMode(mem_dc, TRANSPARENT as _);

                    let count = CURRENT_COUNT.load(Ordering::SeqCst);
                    let title_text = format!("Iniciando gravação em {}…", count);
                    let title_w = to_wide(&title_text);

                    let font_title = CreateFontW(
                        -15, 0, 0, 0,
                        FW_BOLD as _,
                        0, 0, 0, 1, 0, 0, 5, 0,
                        to_wide("Segoe UI").as_ptr(),
                    );
                    let old_font = SelectObject(mem_dc, font_title as _);
                    SetTextColor(mem_dc, rgb(255, 255, 255));
                    let mut title_rc = RECT {
                        left: 48,
                        top: 10,
                        right: rc.right - 14,
                        bottom: 30,
                    };
                    DrawTextW(mem_dc, title_w.as_ptr(), -1, &mut title_rc, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);

                    let font_sub = CreateFontW(
                        -12, 0, 0, 0,
                        FW_NORMAL as _,
                        0, 0, 0, 1, 0, 0, 5, 0,
                        to_wide("Segoe UI").as_ptr(),
                    );
                    SelectObject(mem_dc, font_sub as _);
                    SetTextColor(mem_dc, rgb(165, 172, 190));
                    let sc = SHORTCUT_LABEL.lock().unwrap();
                    let sub_text = if sc.is_empty() {
                        "Pressione o atalho para finalizar".to_string()
                    } else {
                        format!("Pressione {} para finalizar", *sc)
                    };
                    let sub_w = to_wide(&sub_text);
                    let mut sub_rc = RECT {
                        left: 48,
                        top: 31,
                        right: rc.right - 14,
                        bottom: 49,
                    };
                    DrawTextW(mem_dc, sub_w.as_ptr(), -1, &mut sub_rc, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);

                    // Blit memory buffer to window
                    BitBlt(hdc, 0, 0, rc.right, rc.bottom, mem_dc, 0, 0, SRCCOPY);

                    // Cleanup
                    SelectObject(mem_dc, old_font);
                    DeleteObject(font_title as _);
                    DeleteObject(font_sub as _);
                    SelectObject(mem_dc, old_bmp);
                    DeleteObject(mem_bmp as _);
                    DeleteDC(mem_dc);

                    EndPaint(hwnd, &ps);
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn run_countdown_hud(shortcut: &str, flag: &Path) -> Result<bool> {
    *SHORTCUT_LABEL.lock().unwrap() = shortcut.to_string();
    CANCELLED_COUNTDOWN.store(false, Ordering::SeqCst);

    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("BenderShotCountdownHUD");
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(countdown_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0 as _,
            hCursor: 0 as _,
            hbrBackground: 0 as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wnd_class);

        // Detect screen / monitor where the cursor currently is
        let mut pt: POINT = std::mem::zeroed();
        GetCursorPos(&mut pt);
        let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
        let mut minfo: MONITORINFO = std::mem::zeroed();
        minfo.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        let (screen_left, screen_top, screen_w) = if GetMonitorInfoW(hmonitor, &mut minfo) != 0 {
            (
                minfo.rcMonitor.left,
                minfo.rcMonitor.top,
                minfo.rcMonitor.right - minfo.rcMonitor.left,
            )
        } else {
            (0, 0, GetSystemMetrics(SM_CXSCREEN))
        };

        let banner_w = 340;
        let banner_h = 56;
        let x = screen_left + (screen_w - banner_w) / 2;
        let y = screen_top + 28;

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            to_wide("BenderShot Recording Countdown").as_ptr(),
            WS_POPUP,
            x,
            y,
            banner_w,
            banner_h,
            0 as _,
            0 as _,
            hinstance,
            std::ptr::null(),
        );

        if hwnd == 0 as _ {
            return Ok(true);
        }

        let rgn = CreateRoundRectRgn(0, 0, banner_w + 1, banner_h + 1, 22, 22);
        SetWindowRgn(hwnd, rgn, 1);
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        UpdateWindow(hwnd);

        for n in (1..=3).rev() {
            CURRENT_COUNT.store(n, Ordering::SeqCst);
            InvalidateRect(hwnd, std::ptr::null(), 0);
            UpdateWindow(hwnd);

            // Toast notifications during countdown are REMOVED entirely!
            // Only the mini-window HUD on top of the screen displays the countdown.

            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_millis(1000) {
                if flag.exists() || CANCELLED_COUNTDOWN.load(Ordering::SeqCst) {
                    DestroyWindow(hwnd);
                    return Ok(false);
                }
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, 0 as _, 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }

        // Destroy countdown window immediately before starting the video capture
        DestroyWindow(hwnd);
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, 0 as _, 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // Brief delay to allow desktop compositor to refresh before capturing first frame
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(true)
}

pub async fn run_recorder_worker() -> Result<()> {
    let out_video = temp_video_path();
    let flag = stop_flag_file();
    let _ = std::fs::remove_file(&flag);

    let shortcut = crate::platform::windows::shortcuts::ShortcutManager::get_current_record_binding();
    let shortcut_clone = shortcut.clone();
    let flag_clone = flag.clone();

    let proceed = tokio::task::spawn_blocking(move || {
        run_countdown_hud(&shortcut_clone, &flag_clone)
    }).await??;

    if !proceed || flag.exists() {
        let _ = std::fs::remove_file(&flag);
        return Ok(());
    }

    println!("Iniciando gravação de tela no Windows...");
    let _ = std::fs::write(video_path_file(), out_video.to_string_lossy().as_bytes());

    // Spawn ffmpeg with gdigrab desktop device
    use std::os::windows::process::CommandExt;
    let ffmpeg = Command::new(crate::video::ScreenRecorder::ffmpeg_bin())
        .creation_flags(0x08000000)
        .args([
            "-y",
            "-f", "gdigrab",
            "-framerate", "30",
            "-i", "desktop",
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-pix_fmt", "yuv420p",
            out_video.to_str().unwrap_or("bendershot-rec.mp4"),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut ffmpeg = ffmpeg;
    while !flag.exists() {
        // Bail out if ffmpeg failed to start or died, instead of hanging forever.
        match ffmpeg.as_mut() {
            Ok(child) => {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
            }
            Err(_) => break,
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    if let Ok(mut child) = ffmpeg {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"q\n");
        }
        let _ = child.wait();
    }
    let _ = std::fs::remove_file(&flag);
    Ok(())
}
