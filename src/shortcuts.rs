#[derive(Debug, Clone)]
pub struct KeybindingConflict {
    pub schema: String,
    pub key: String,
    pub binding: String,
    pub description: String,
}

pub struct ShortcutManager;

impl ShortcutManager {
    pub fn detect_conflict(candidate_shortcut: &str) -> Option<KeybindingConflict> {
        crate::platform::imp::shortcuts::ShortcutManager::detect_conflict(candidate_shortcut)
    }

    pub fn get_current_binding() -> String {
        crate::platform::imp::shortcuts::ShortcutManager::get_current_binding()
    }

    pub fn set_binding(new_binding: &str) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::set_binding(new_binding)
    }

    pub fn get_current_window_binding() -> String {
        crate::platform::imp::shortcuts::ShortcutManager::get_current_window_binding()
    }

    pub fn set_window_binding(new_binding: &str) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::set_window_binding(new_binding)
    }

    pub fn get_current_region_binding() -> String {
        crate::platform::imp::shortcuts::ShortcutManager::get_current_region_binding()
    }

    pub fn set_region_binding(new_binding: &str) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::set_region_binding(new_binding)
    }

    pub fn get_current_record_binding() -> String {
        crate::platform::imp::shortcuts::ShortcutManager::get_current_record_binding()
    }

    pub fn set_record_binding(new_binding: &str) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::set_record_binding(new_binding)
    }

    pub fn disable_gnome_screenshot_ui() -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::disable_gnome_screenshot_ui()
    }

    pub fn restore_gnome_screenshot_ui() -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::restore_gnome_screenshot_ui()
    }

    pub fn is_gnome_screenshot_ui_disabled() -> bool {
        crate::platform::imp::shortcuts::ShortcutManager::is_gnome_screenshot_ui_disabled()
    }

    pub fn is_shortcut_registered() -> bool {
        crate::platform::imp::shortcuts::ShortcutManager::is_shortcut_registered()
    }

    pub fn configure_autostart(enable: bool) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::configure_autostart(enable)
    }

    pub fn is_autostart_enabled() -> bool {
        crate::platform::imp::shortcuts::ShortcutManager::is_autostart_enabled()
    }

    pub fn start_daemon_if_not_running() -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::start_daemon_if_not_running()
    }

    pub fn is_running_as_appimage() -> bool {
        crate::platform::imp::shortcuts::ShortcutManager::is_running_as_appimage()
    }

    pub fn integrate_appimage() -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::integrate_appimage()
    }

    pub fn send_notification(summary: &str, body: &str) {
        crate::platform::imp::shortcuts::ShortcutManager::send_notification(summary, body);
    }
}
