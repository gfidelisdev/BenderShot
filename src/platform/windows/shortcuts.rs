use std::path::PathBuf;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use crate::shortcuts::KeybindingConflict;

#[derive(Serialize, Deserialize, Default)]
struct SavedShortcuts {
    capture: Option<String>,
    #[serde(default)]
    window: Option<String>,
    #[serde(default)]
    region: Option<String>,
    record: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    let proj = ProjectDirs::from("com", "bendershot", "bendershot")?;
    Some(proj.config_dir().join("shortcuts.json"))
}

fn read_saved_shortcuts() -> SavedShortcuts {
    if let Some(path) = config_path() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(saved) = serde_json::from_str::<SavedShortcuts>(&content) {
                return saved;
            }
        }
    }
    SavedShortcuts::default()
}

fn write_saved_shortcuts(saved: &SavedShortcuts) -> Result<(), String> {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(saved) {
            return std::fs::write(path, json).map_err(|e| e.to_string());
        }
    }
    Ok(())
}

pub struct ShortcutManager;

impl ShortcutManager {
    /// Detects Windows keybinding conflicts
    pub fn detect_conflict(candidate_shortcut: &str) -> Option<KeybindingConflict> {
        let norm = normalize_shortcut(candidate_shortcut);
        if norm.is_empty() {
            return None;
        }

        // Known Windows system reserved shortcuts
        let reserved = [
            ("ctrl+alt+del", "Atalho Reservado do Sistema Windows"),
            ("win+l", "Bloquear Estação de Trabalho do Windows"),
            ("win+d", "Exibir Área de Trabalho"),
            ("win+e", "Abrir Explorador de Arquivos"),
        ];

        for (res, desc) in reserved {
            if norm == res {
                return Some(KeybindingConflict {
                    schema: "windows.system".to_string(),
                    key: res.to_string(),
                    binding: candidate_shortcut.to_string(),
                    description: desc.to_string(),
                });
            }
        }

        None
    }

    /// Retrieves current BenderShot keybinding on Windows (defaults to Ctrl+Print)
    pub fn get_current_binding() -> String {
        let saved = read_saved_shortcuts();
        match saved.capture {
            Some(ref s) if s.trim().eq_ignore_ascii_case("print") => "Ctrl+Print".to_string(),
            Some(s) if !s.is_empty() => s,
            _ => "Ctrl+Print".to_string(),
        }
    }

    /// Sets BenderShot capture keybinding on Windows
    pub fn set_binding(new_binding: &str) -> Result<(), String> {
        let mut saved = read_saved_shortcuts();
        saved.capture = Some(new_binding.to_string());
        let res = write_saved_shortcuts(&saved);
        notify_tray_daemon_reload();
        res
    }

    /// Retrieves current active window keybinding on Windows
    pub fn get_current_window_binding() -> String {
        let saved = read_saved_shortcuts();
        saved.window.unwrap_or_else(|| "Alt+Print".to_string())
    }

    /// Sets active window keybinding on Windows
    pub fn set_window_binding(new_binding: &str) -> Result<(), String> {
        let mut saved = read_saved_shortcuts();
        saved.window = Some(new_binding.to_string());
        let res = write_saved_shortcuts(&saved);
        notify_tray_daemon_reload();
        res
    }

    /// Retrieves current region keybinding on Windows
    pub fn get_current_region_binding() -> String {
        let saved = read_saved_shortcuts();
        saved.region.unwrap_or_else(|| "Shift+Print".to_string())
    }

    /// Sets region keybinding on Windows
    pub fn set_region_binding(new_binding: &str) -> Result<(), String> {
        let mut saved = read_saved_shortcuts();
        saved.region = Some(new_binding.to_string());
        let res = write_saved_shortcuts(&saved);
        notify_tray_daemon_reload();
        res
    }

    /// Retrieves current video recording keybinding on Windows
    pub fn get_current_record_binding() -> String {
        let saved = read_saved_shortcuts();
        saved.record.unwrap_or_else(|| "Ctrl+Shift+R".to_string())
    }

    /// Sets video recording keybinding on Windows
    pub fn set_record_binding(new_binding: &str) -> Result<(), String> {
        let mut saved = read_saved_shortcuts();
        saved.record = Some(new_binding.to_string());
        let res = write_saved_shortcuts(&saved);
        notify_tray_daemon_reload();
        res
    }

    /// Parses shortcut string into Win32 (modifiers, vk_code)
    pub fn parse_hotkey(s: &str) -> Option<(u32, u32)> {
        let norm = s.to_lowercase()
            .replace('<', "+")
            .replace('>', "+")
            .replace("control", "ctrl")
            .replace("primary", "ctrl");

        let parts: Vec<&str> = norm.split('+')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();
        let mut mods = windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_NOREPEAT;
        let mut vk = 0u32;

        for part in parts {
            match part {
                "ctrl" => mods |= windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL,
                "shift" => mods |= windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_SHIFT,
                "alt" => mods |= windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_ALT,
                "win" | "super" => mods |= windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_WIN,
                "print" | "printscreen" | "prtscn" | "snapshot" => {
                    vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SNAPSHOT as u32;
                }
                "space" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SPACE as u32,
                "insert" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_INSERT as u32,
                "delete" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_DELETE as u32,
                "f1" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F1 as u32,
                "f2" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F2 as u32,
                "f3" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F3 as u32,
                "f4" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4 as u32,
                "f5" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F5 as u32,
                "f6" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F6 as u32,
                "f7" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F7 as u32,
                "f8" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F8 as u32,
                "f9" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F9 as u32,
                "f10" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F10 as u32,
                "f11" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F11 as u32,
                "f12" => vk = windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F12 as u32,
                single if single.len() == 1 => {
                    let ch = single.chars().next().unwrap();
                    if ch.is_ascii_alphanumeric() {
                        vk = ch.to_ascii_uppercase() as u32;
                    }
                }
                _ => {}
            }
        }

        if vk != 0 {
            Some((mods, vk))
        } else {
            None
        }
    }

    pub fn disable_gnome_screenshot_ui() -> Result<(), String> {
        Ok(())
    }

    pub fn restore_gnome_screenshot_ui() -> Result<(), String> {
        Ok(())
    }

    pub fn is_gnome_screenshot_ui_disabled() -> bool {
        true
    }

    pub fn is_shortcut_registered() -> bool {
        true
    }

    pub fn configure_autostart(_enable: bool) -> Result<(), String> {
        Ok(())
    }

    pub fn is_autostart_enabled() -> bool {
        true
    }

    pub fn start_daemon_if_not_running() -> Result<(), String> {
        Ok(())
    }

    pub fn is_running_as_appimage() -> bool {
        false
    }

    pub fn integrate_appimage() -> Result<(), String> {
        Ok(())
    }

    pub fn send_notification(_summary: &str, _body: &str) {
    }
}

fn notify_tray_daemon_reload() {
    unsafe {
        let class_name: Vec<u16> = "BenderShotTrayClass"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(class_name.as_ptr(), std::ptr::null());
        if hwnd != 0 as _ {
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                hwnd,
                crate::platform::windows::indicator::WM_RELOAD_SHORTCUTS,
                0,
                0,
            );
        }
    }
}

fn normalize_shortcut(s: &str) -> String {
    s.to_lowercase()
        .replace('<', "")
        .replace('>', "")
        .replace("control", "ctrl")
        .replace("primary", "ctrl")
        .trim()
        .to_string()
}
