use std::path::PathBuf;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use crate::shortcuts::KeybindingConflict;

#[derive(Serialize, Deserialize, Default)]
struct SavedShortcuts {
    capture: Option<String>,
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

    /// Retrieves current BenderShot keybinding on Windows
    pub fn get_current_binding() -> String {
        let saved = read_saved_shortcuts();
        saved.capture.unwrap_or_else(|| "Print".to_string())
    }

    /// Sets BenderShot capture keybinding on Windows
    pub fn set_binding(new_binding: &str) -> Result<(), String> {
        let mut saved = read_saved_shortcuts();
        saved.capture = Some(new_binding.to_string());
        write_saved_shortcuts(&saved)
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
        write_saved_shortcuts(&saved)
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
