use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::null;
use crate::capture::CaptureMode;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadIconW, LoadImageW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, ShowWindow, TrackPopupMenu, TranslateMessage, HICON, IDI_APPLICATION,
    IMAGE_ICON, LR_LOADFROMFILE, MF_SEPARATOR, MF_STRING, MSG, SW_HIDE, TPM_BOTTOMALIGN,
    TPM_RIGHTALIGN, WM_APP, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDBLCLK, WM_RBUTTONUP,
    WNDCLASSW,
};

pub const WM_TRAYICON: u32 = WM_APP + 100;
pub const WM_RELOAD_SHORTCUTS: u32 = WM_APP + 101;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const ID_TRAY_REGION: usize = 1001;
const ID_TRAY_FULL: usize = 1002;
const ID_TRAY_WINDOW: usize = 1003;
const ID_TRAY_RECORD: usize = 1004;
const ID_TRAY_SETTINGS: usize = 1005;
const ID_TRAY_UPDATE: usize = 1006;
const ID_TRAY_EXIT: usize = 1007;

const HOTKEY_FULL_ID: i32 = 1;
const HOTKEY_CAPTURE_FALLBACK_ID: i32 = 2;
const HOTKEY_RECORD_ID: i32 = 3;
const HOTKEY_WINDOW_ID: i32 = 4;
const HOTKEY_REGION_ID: i32 = 5;

static EMBEDDED_ICON: &[u8] = include_bytes!("../../../packaging/assets/bendershot.ico");

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn hide_console_window() {
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd != 0 as _ {
            ShowWindow(hwnd, SW_HIDE);
        }
        FreeConsole();
    }
}

pub fn spawn_capture(mode: CaptureMode) {
    if let Ok(exe) = std::env::current_exe() {
        let arg = match mode {
            CaptureMode::Region => "region",
            CaptureMode::Fullscreen => "full",
            CaptureMode::Window => "window",
        };
        let _ = Command::new(exe)
            .arg(arg)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

fn spawn_record() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg("record")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

fn spawn_settings() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg("settings")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

fn spawn_update() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .args(["update", "--gui"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

fn load_tray_icon() -> HICON {
    let mut candidate_paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidate_paths.push(dir.join("bendershot.ico"));
            candidate_paths.push(dir.join("assets").join("bendershot.ico"));
        }
    }
    candidate_paths.push(PathBuf::from("packaging/assets/bendershot.ico"));

    if let Some(proj) = directories::ProjectDirs::from("com", "bendershot", "bendershot") {
        let cache_ico = proj.config_dir().join("bendershot.ico");
        if !cache_ico.exists() {
            let _ = std::fs::create_dir_all(proj.config_dir());
            let _ = std::fs::write(&cache_ico, EMBEDDED_ICON);
        }
        candidate_paths.push(cache_ico);
    }

    for path in candidate_paths {
        if path.exists() {
            let wide = to_wide(&path.to_string_lossy());
            unsafe {
                let h = LoadImageW(0 as _, wide.as_ptr(), IMAGE_ICON, 32, 32, LR_LOADFROMFILE);
                if h != 0 as _ {
                    return h as HICON;
                }
            }
        }
    }

    unsafe { LoadIconW(0 as _, IDI_APPLICATION) }
}

unsafe fn register_all_hotkeys(hwnd: HWND) {
    unsafe {
        UnregisterHotKey(hwnd, HOTKEY_FULL_ID);
        UnregisterHotKey(hwnd, HOTKEY_WINDOW_ID);
        UnregisterHotKey(hwnd, HOTKEY_REGION_ID);
        UnregisterHotKey(hwnd, HOTKEY_RECORD_ID);
        UnregisterHotKey(hwnd, HOTKEY_CAPTURE_FALLBACK_ID);

        let mut registered_combos: Vec<(u32, u32)> = Vec::new();

        // 1. Fullscreen Capture
        let cap_str = crate::platform::windows::shortcuts::ShortcutManager::get_current_binding();
        if let Some((mods, vk)) = crate::platform::windows::shortcuts::ShortcutManager::parse_hotkey(&cap_str) {
            let res = RegisterHotKey(hwnd, HOTKEY_FULL_ID, mods, vk);
            if res != 0 {
                registered_combos.push((mods, vk));
                tracing::info!("Registered full capture hotkey: {}", cap_str);
            } else {
                tracing::warn!(
                    "Failed to register full capture hotkey '{}' (may be held by Windows Snipping Tool or another app)",
                    cap_str
                );
            }
        }

        // 2. Active Window Capture
        let win_str = crate::platform::windows::shortcuts::ShortcutManager::get_current_window_binding();
        if let Some((mods, vk)) = crate::platform::windows::shortcuts::ShortcutManager::parse_hotkey(&win_str) {
            if !registered_combos.contains(&(mods, vk)) {
                let res = RegisterHotKey(hwnd, HOTKEY_WINDOW_ID, mods, vk);
                if res != 0 {
                    registered_combos.push((mods, vk));
                    tracing::info!("Registered window capture hotkey: {}", win_str);
                } else {
                    tracing::warn!("Failed to register window capture hotkey '{}'", win_str);
                }
            } else {
                tracing::warn!("Window capture hotkey '{}' duplicates another registered hotkey", win_str);
            }
        }

        // 3. Region Capture
        let reg_str = crate::platform::windows::shortcuts::ShortcutManager::get_current_region_binding();
        if let Some((mods, vk)) = crate::platform::windows::shortcuts::ShortcutManager::parse_hotkey(&reg_str) {
            if !registered_combos.contains(&(mods, vk)) {
                let res = RegisterHotKey(hwnd, HOTKEY_REGION_ID, mods, vk);
                if res != 0 {
                    registered_combos.push((mods, vk));
                    tracing::info!("Registered region capture hotkey: {}", reg_str);
                } else {
                    tracing::warn!("Failed to register region capture hotkey '{}'", reg_str);
                }
            } else {
                tracing::warn!("Region capture hotkey '{}' duplicates another registered hotkey", reg_str);
            }
        }

        // 4. Video Recording
        let rec_str = crate::platform::windows::shortcuts::ShortcutManager::get_current_record_binding();
        if let Some((mods, vk)) = crate::platform::windows::shortcuts::ShortcutManager::parse_hotkey(&rec_str) {
            if !registered_combos.contains(&(mods, vk)) {
                let res = RegisterHotKey(hwnd, HOTKEY_RECORD_ID, mods, vk);
                if res != 0 {
                    registered_combos.push((mods, vk));
                    tracing::info!("Registered video record hotkey: {}", rec_str);
                } else {
                    tracing::warn!("Failed to register video record hotkey '{}'", rec_str);
                }
            } else {
                tracing::warn!("Video record hotkey '{}' duplicates another registered hotkey", rec_str);
            }
        }

        // 5. Fallback Ctrl+Shift+S (Region capture) if not already registered
        let fallback_parsed = crate::platform::windows::shortcuts::ShortcutManager::parse_hotkey("Ctrl+Shift+S");
        if let Some((fb_mods, fb_vk)) = fallback_parsed {
            if !registered_combos.contains(&(fb_mods, fb_vk)) {
                let res = RegisterHotKey(hwnd, HOTKEY_CAPTURE_FALLBACK_ID, fb_mods, fb_vk);
                if res != 0 {
                    tracing::info!("Registered fallback region capture hotkey: Ctrl+Shift+S");
                }
            }
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_TRAYICON => {
                let event = lparam as u32;
                match event {
                    WM_RBUTTONUP => {
                        let hmenu = CreatePopupMenu();
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_REGION, to_wide("📸 Capturar Região").as_ptr());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_FULL, to_wide("🖥️ Capturar Tela Cheia").as_ptr());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_WINDOW, to_wide("🪟 Capturar Janela").as_ptr());
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_RECORD, to_wide("🎬 Gravar Tela (Vídeo)").as_ptr());
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_SETTINGS, to_wide("⚙️ Configurações").as_ptr());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_UPDATE, to_wide("🔄 Verificar Atualizações").as_ptr());
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
                        AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT, to_wide("❌ Sair").as_ptr());

                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        SetForegroundWindow(hwnd);
                        TrackPopupMenu(hmenu, TPM_RIGHTALIGN | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, null());
                        DestroyMenu(hmenu);
                    }
                    WM_LBUTTONDBLCLK => {
                        spawn_capture(CaptureMode::Region);
                    }
                    _ => {}
                }
                0
            }
            WM_COMMAND => {
                let id = (wparam & 0xFFFF) as usize;
                match id {
                    ID_TRAY_REGION => spawn_capture(CaptureMode::Region),
                    ID_TRAY_FULL => spawn_capture(CaptureMode::Fullscreen),
                    ID_TRAY_WINDOW => spawn_capture(CaptureMode::Window),
                    ID_TRAY_RECORD => spawn_record(),
                    ID_TRAY_SETTINGS => spawn_settings(),
                    ID_TRAY_UPDATE => spawn_update(),
                    ID_TRAY_EXIT => {
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                0
            }
            WM_HOTKEY => {
                let id = wparam as i32;
                match id {
                    HOTKEY_FULL_ID => {
                        spawn_capture(CaptureMode::Fullscreen);
                    }
                    HOTKEY_WINDOW_ID => {
                        spawn_capture(CaptureMode::Window);
                    }
                    HOTKEY_REGION_ID | HOTKEY_CAPTURE_FALLBACK_ID => {
                        spawn_capture(CaptureMode::Region);
                    }
                    HOTKEY_RECORD_ID => {
                        spawn_record();
                    }
                    _ => {}
                }
                0
            }
            WM_RELOAD_SHORTCUTS => {
                register_all_hotkeys(hwnd);
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

pub async fn run_indicator() -> anyhow::Result<()> {
    tracing::info!("Starting BenderShot Windows tray daemon...");

    // 1. Hide terminal console window immediately
    hide_console_window();

    // 2. Run Win32 message loop in dedicated OS thread
    tokio::task::spawn_blocking(move || {
        unsafe {
            let class_name = to_wide("BenderShotTrayClass");
            let wnd_class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as _,
                hIcon: 0 as _,
                hCursor: 0 as _,
                hbrBackground: 0 as _,
                lpszMenuName: null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                to_wide("BenderShotTrayWindow").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                0 as _,
                0 as _,
                0 as _,
                null(),
            );

            if hwnd == 0 as _ {
                tracing::error!("Failed to create hidden window for BenderShot tray");
                return;
            }

            let hicon = load_tray_icon();

            // 3. Register System Tray icon
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = hicon;

            let tip = to_wide("BenderShot");
            let copy_len = tip.len().min(nid.szTip.len() - 1);
            nid.szTip[..copy_len].copy_from_slice(&tip[..copy_len]);

            Shell_NotifyIconW(NIM_ADD, &nid);

            // 4. Register Global Hotkeys
            register_all_hotkeys(hwnd);

            // 5. Message Loop
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 6. Cleanup on exit
            Shell_NotifyIconW(NIM_DELETE, &nid);
            UnregisterHotKey(hwnd, HOTKEY_FULL_ID);
            UnregisterHotKey(hwnd, HOTKEY_WINDOW_ID);
            UnregisterHotKey(hwnd, HOTKEY_REGION_ID);
            UnregisterHotKey(hwnd, HOTKEY_RECORD_ID);
            UnregisterHotKey(hwnd, HOTKEY_CAPTURE_FALLBACK_ID);
        }
    })
    .await?;

    Ok(())
}
