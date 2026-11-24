use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolbarPosition {
    #[default]
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub setup_completed: bool,
    pub autostart_enabled: bool,
    pub capture_shortcut: String,
    pub record_shortcut: String,
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default)]
    pub last_update_check_epoch: u64,
    #[serde(default)]
    pub latest_known_tag: Option<String>,
    #[serde(default)]
    pub installed_version_override: Option<String>,
    #[serde(default)]
    pub toolbar_position: ToolbarPosition,
    #[serde(default)]
    pub toolbar_expanded: bool,
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
            auto_check_updates: true,
            last_update_check_epoch: 0,
            latest_known_tag: None,
            installed_version_override: None,
            toolbar_position: ToolbarPosition::Left,
            toolbar_expanded: false,
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

    pub fn effective_version_from(&self, pkg_version: &str) -> String {
        if let Some(ref installed) = self.installed_version_override {
            let trimmed = installed.trim();
            if !trimmed.is_empty() && crate::updater::is_newer_version(pkg_version, trimmed) {
                return trimmed.to_string();
            }
        }
        pkg_version.to_string()
    }

    pub fn get_effective_version() -> String {
        Self::load().effective_version_from(env!("CARGO_PKG_VERSION"))
    }

    pub fn set_installed_version(tag: &str) {
        let mut cfg = Self::load();
        cfg.installed_version_override = Some(tag.to_string());
        cfg.latest_known_tag = None;
        let _ = cfg.save();
    }

    pub fn record_update_check(latest_tag: Option<String>) {
        let mut cfg = Self::load();
        cfg.last_update_check_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        cfg.latest_known_tag = latest_tag;
        let _ = cfg.save();
    }
}
