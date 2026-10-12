use std::process::Command;

#[derive(Debug, Clone)]
pub struct KeybindingConflict {
    pub schema: String,
    pub key: String,
    pub binding: String,
    pub description: String,
}

pub struct ShortcutManager;

impl ShortcutManager {
    /// Detects current desktop environment keybindings and flags conflicts with `candidate_shortcut`
    pub fn detect_conflict(candidate_shortcut: &str) -> Option<KeybindingConflict> {
        let norm_candidate = normalize_shortcut(candidate_shortcut);
        if norm_candidate.is_empty() {
            return None;
        }

        // 1. Check GNOME shell keybindings
        let schemas = [
            ("org.gnome.shell.keybindings", "GNOME Shell"),
            ("org.gnome.desktop.wm.keybindings", "Gerenciador de Janelas GNOME"),
            ("org.gnome.settings-daemon.plugins.media-keys", "Teclas de Mídia GNOME"),
        ];

        for (schema, desc) in schemas {
            if let Ok(output) = Command::new("gsettings")
                .args(["list-recursively", schema])
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let key = parts[1];
                            let raw_val = parts[2..].join(" ");
                            if raw_val.contains(&format!("'{}'", candidate_shortcut))
                                || raw_val.contains(&format!("\"{}\"", candidate_shortcut))
                            {
                                // Skip our own enhanced-shot binding
                                if line.contains("enhanced-shot") {
                                    continue;
                                }
                                return Some(KeybindingConflict {
                                    schema: schema.to_string(),
                                    key: key.to_string(),
                                    binding: candidate_shortcut.to_string(),
                                    description: desc.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 2. Check existing custom keybindings in GNOME
        if let Ok(output) = Command::new("gsettings")
            .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                // e.g. ['/path1/', '/path2/']
                for path in text.split('\'').filter(|s| s.starts_with('/')) {
                    if path.contains("enhanced-shot") {
                        continue;
                    }
                    if let Ok(b_out) = Command::new("gsettings")
                        .args([
                            "get",
                            &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path),
                            "binding",
                        ])
                        .output()
                    {
                        let b_val = String::from_utf8_lossy(&b_out.stdout).trim().trim_matches('\'').to_string();
                        if normalize_shortcut(&b_val) == norm_candidate {
                            let name_out = Command::new("gsettings")
                                .args([
                                    "get",
                                    &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path),
                                    "name",
                                ])
                                .output()
                                .ok();
                            let name = name_out
                                .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_matches('\'').to_string())
                                .unwrap_or_else(|| "Atalho Personalizado".into());
                            return Some(KeybindingConflict {
                                schema: "custom-keybindings".to_string(),
                                key: name,
                                binding: b_val,
                                description: "Atalho personalizado de outro aplicativo".to_string(),
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Retrieves current Enhanced Shot keybinding registered in the DE
    pub fn get_current_binding() -> String {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/";
        if let Ok(out) = Command::new("gsettings")
            .args([
                "get",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "binding",
            ])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string();
                if !s.is_empty() && s != "@as []" {
                    return s;
                }
            }
        }
        "Print".to_string()
    }

    /// Sets Enhanced Shot shortcut in GNOME
    pub fn set_binding(new_binding: &str) -> Result<(), String> {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/";

        if new_binding == "Print" {
            // Disable GNOME native UI to prevent conflict
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.shell.keybindings", "show-screenshot-ui", "[]"])
                .output();
        }

        // Register custom-keybinding slot
        let current_bindings = Command::new("gsettings")
            .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "[]".into());

        if !current_bindings.contains(custom_path) {
            let new_list = if current_bindings == "@as []" || current_bindings == "[]" {
                format!("['{}']", custom_path)
            } else {
                format!("{}, '{}']", current_bindings.trim_end_matches(']'), custom_path)
            };
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", &new_list])
                .output();
        }

        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "enhanced-shot".to_string());
        let cmd = format!("{} region", exe);
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "name",
                "Enhanced Shot Captura",
            ])
            .output();
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "command",
                &cmd,
            ])
            .output();
        let status = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "binding",
                new_binding,
            ])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Falha ao executar gsettings".into())
        }
    }

    /// Retrieves current Enhanced Shot video recording keybinding
    pub fn get_current_record_binding() -> String {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot-record/";
        if let Ok(out) = Command::new("gsettings")
            .args([
                "get",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "binding",
            ])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string();
                if !s.is_empty() && s != "@as []" {
                    return s;
                }
            }
        }
        "<Ctrl><Shift>R".to_string()
    }

    /// Sets Enhanced Shot video recording shortcut in GNOME
    pub fn set_record_binding(new_binding: &str) -> Result<(), String> {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot-record/";

        let current_bindings = Command::new("gsettings")
            .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "[]".into());

        if !current_bindings.contains(custom_path) {
            let new_list = if current_bindings == "@as []" || current_bindings == "[]" {
                format!("['{}']", custom_path)
            } else {
                format!("{}, '{}']", current_bindings.trim_end_matches(']'), custom_path)
            };
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", &new_list])
                .output();
        }

        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "enhanced-shot".to_string());
        let cmd = format!("{} record", exe);
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "name",
                "Enhanced Shot Gravar Vídeo",
            ])
            .output();
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "command",
                &cmd,
            ])
            .output();
        let status = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "binding",
                new_binding,
            ])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Falha ao executar gsettings para atalho de gravação".into())
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
