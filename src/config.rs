use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub setup_completed: bool,
    pub autostart_enabled: bool,
    pub capture_shortcut: String,
    pub record_shortcut: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            setup_completed: false,
            autostart_enabled: true,
            #[cfg(target_os = "linux")]
            capture_shortcut: "Print".to_string(),
            #[cfg(target_os = "windows")]
            capture_shortcut: "<Ctrl>Print".to_string(),
            record_shortcut: "<Ctrl><Shift>R".to_string(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        let proj = ProjectDirs::from("org", "bendershot", "bendershot")?;
        Some(proj.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn is_setup_completed() -> bool {
        Self::load().setup_completed
    }

    pub fn mark_setup_completed() {
        let mut cfg = Self::load();
        cfg.setup_completed = true;
        let _ = cfg.save();
    }
}
