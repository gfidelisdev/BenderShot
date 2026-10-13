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

    pub fn get_current_record_binding() -> String {
        crate::platform::imp::shortcuts::ShortcutManager::get_current_record_binding()
    }

    pub fn set_record_binding(new_binding: &str) -> Result<(), String> {
        crate::platform::imp::shortcuts::ShortcutManager::set_record_binding(new_binding)
    }
}
