use std::process::Command;
use crate::shortcuts::KeybindingConflict;

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
                                // Skip our own bendershot / enhanced-shot binding
                                if line.contains("bendershot") || line.contains("enhanced-shot") {
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
                    if path.contains("bendershot") || path.contains("enhanced-shot") {
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

    /// Retrieves current BenderShot keybinding registered in the DE
    pub fn get_current_binding() -> String {
        for custom_path in [
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/bendershot/",
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/",
        ] {
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
        }
        "Print".to_string()
    }

    /// Sets BenderShot shortcut in GNOME
    pub fn set_binding(new_binding: &str) -> Result<(), String> {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/bendershot/";

        // 1. Clean up legacy bindings (e.g. enhanced-shot) and conflicting duplicates
        cleanup_legacy_keybindings();
        unbind_conflicting_custom_shortcuts(new_binding, custom_path);

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

        let exe = get_executable_command();
        let cmd = format!("{} region", exe);
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "name",
                "BenderShot Captura",
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

    /// Disables native GNOME screenshot UI to prevent conflict
    pub fn disable_gnome_screenshot_ui() -> Result<(), String> {
        let status = Command::new("gsettings")
            .args(["set", "org.gnome.shell.keybindings", "show-screenshot-ui", "[]"])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Falha ao desativar atalho nativo do GNOME".into())
        }
    }

    /// Restores native GNOME screenshot UI
    pub fn restore_gnome_screenshot_ui() -> Result<(), String> {
        let status = Command::new("gsettings")
            .args(["set", "org.gnome.shell.keybindings", "show-screenshot-ui", "['Print']"])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Falha ao restaurar atalho nativo do GNOME".into())
        }
    }

    /// Checks if GNOME native screenshot UI is disabled
    pub fn is_gnome_screenshot_ui_disabled() -> bool {
        if let Ok(output) = Command::new("gsettings")
            .args(["get", "org.gnome.shell.keybindings", "show-screenshot-ui"])
            .output()
        {
            if output.status.success() {
                let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return val == "@as []" || val == "[]";
            }
        }
        false
    }

    /// Checks if BenderShot is already registered in GNOME custom-keybindings
    pub fn is_shortcut_registered() -> bool {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/bendershot/";
        if let Ok(output) = Command::new("gsettings")
            .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
            .output()
        {
            if output.status.success() {
                let val = String::from_utf8_lossy(&output.stdout);
                return val.contains(custom_path);
            }
        }
        false
    }

    /// Configures autostart via systemd user service and XDG autostart desktop entry
    pub fn configure_autostart(enable: bool) -> Result<(), String> {
        if enable {
            let _ = Command::new("systemctl")
                .args(["--user", "enable", "bendershot.service"])
                .output();
        } else {
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "bendershot.service"])
                .output();
        }

        if let Some(base_dirs) = directories::BaseDirs::new() {
            let autostart_dir = base_dirs.config_dir().join("autostart");
            let desktop_file = autostart_dir.join("bendershot.desktop");

            if enable {
                let _ = std::fs::create_dir_all(&autostart_dir);
                let exe_cmd = get_executable_command();
                let content = format!(
                    "[Desktop Entry]\nType=Application\nName=BenderShot\nComment=BenderShot daemon e atalhos na bandeja\nExec={} daemon\nIcon=bendershot\nTerminal=false\nCategories=Utility;\nX-GNOME-Autostart-enabled=true\n",
                    exe_cmd
                );
                let _ = std::fs::write(desktop_file, content);
            } else {
                let _ = std::fs::remove_file(desktop_file);
            }
        }
        Ok(())
    }

    /// Checks if autostart is enabled
    pub fn is_autostart_enabled() -> bool {
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let desktop_file = base_dirs.config_dir().join("autostart").join("bendershot.desktop");
            if desktop_file.exists() {
                return true;
            }
        }
        if let Ok(output) = Command::new("systemctl")
            .args(["--user", "is-enabled", "bendershot.service"])
            .output()
        {
            if output.status.success() {
                let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return val == "enabled";
            }
        }
        false
    }

    /// Starts the daemon service if not currently running
    pub fn start_daemon_if_not_running() -> Result<(), String> {
        let sys_out = Command::new("systemctl")
            .args(["--user", "is-active", "bendershot.service"])
            .output();
        if let Ok(out) = sys_out {
            if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "active" {
                return Ok(());
            }
            if let Ok(start_out) = Command::new("systemctl")
                .args(["--user", "start", "bendershot.service"])
                .output()
            {
                if start_out.status.success() {
                    return Ok(());
                }
            }
        }

        let exe = get_executable_command();
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let mut cmd = Command::new(exe);
            cmd.arg("daemon")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .process_group(0);
            cmd.spawn().map_err(|e| format!("Falha ao iniciar daemon: {}", e))?;
        }
        #[cfg(not(unix))]
        {
            Command::new(exe)
                .arg("daemon")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Falha ao iniciar daemon: {}", e))?;
        }

        Ok(())
    }

    /// Checks if current process is running inside an AppImage
    pub fn is_running_as_appimage() -> bool {
        std::env::var("APPIMAGE").map(|s| !s.trim().is_empty()).unwrap_or(false)
    }

    /// Integrates AppImage with the desktop environment
    pub fn integrate_appimage() -> Result<(), String> {
        let appimage_path = std::env::var("APPIMAGE").map_err(|_| "Não está executando como AppImage".to_string())?;
        let base_dirs = directories::BaseDirs::new().ok_or("Falha ao obter diretórios do usuário".to_string())?;
        let data_dir = base_dirs.data_dir(); // ~/.local/share

        let apps_dir = data_dir.join("applications");
        let _ = std::fs::create_dir_all(&apps_dir);
        let desktop_file = apps_dir.join("bendershot.desktop");

        let desktop_content = format!(
            "[Desktop Entry]\nName=BenderShot\nComment=Captura, gravação e anotação profissional de tela para Linux Wayland\nExec={} region\nIcon=bendershot\nTerminal=false\nType=Application\nCategories=Utility;Graphics;Photography;\nKeywords=screenshot;captura;print;tela;anotação;gravação;video;screencast;\nActions=Region;Full;Window;Record;Daemon;Settings;Wizard;\n\n[Desktop Action Region]\nName=Capturar Região\nExec={} region\n\n[Desktop Action Full]\nName=Capturar Tela Inteira\nExec={} full\n\n[Desktop Action Window]\nName=Capturar Janela Ativa\nExec={} window\n\n[Desktop Action Record]\nName=Gravar Vídeo da Tela\nExec={} record\n\n[Desktop Action Daemon]\nName=Iniciar no Painel Superior (Daemon)\nExec={} daemon\n\n[Desktop Action Settings]\nName=Configurações\nExec={} settings\n\n[Desktop Action Wizard]\nName=Assistente de Configuração\nExec={} wizard\n",
            appimage_path, appimage_path, appimage_path, appimage_path, appimage_path, appimage_path, appimage_path, appimage_path
        );
        let _ = std::fs::write(&desktop_file, desktop_content);

        let icon_dir = data_dir.join("icons").join("hicolor").join("scalable").join("apps");
        let _ = std::fs::create_dir_all(&icon_dir);
        static SVG_BYTES: &[u8] = include_bytes!("../../../packaging/assets/bendershot.svg");
        let icon_file = icon_dir.join("bendershot.svg");
        let _ = std::fs::write(icon_file, SVG_BYTES);

        let home = base_dirs.home_dir();
        let local_bin = home.join(".local").join("bin");
        let _ = std::fs::create_dir_all(&local_bin);
        let target_symlink = local_bin.join("bendershot");
        let _ = std::fs::remove_file(&target_symlink);
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&appimage_path, &target_symlink);

        let _ = Command::new("update-desktop-database").arg(&apps_dir).output();
        let _ = Command::new("gtk-update-icon-cache").args(["-f", "-t", data_dir.join("icons").join("hicolor").to_str().unwrap_or("")]).output();

        Ok(())
    }

    /// Sends a desktop notification using notify-send
    pub fn send_notification(summary: &str, body: &str) {
        let _ = Command::new("notify-send")
            .args(["-a", "BenderShot", "-i", "bendershot", summary, body])
            .output();
    }

    /// Retrieves current BenderShot video recording keybinding
    pub fn get_current_record_binding() -> String {
        for custom_path in [
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/bendershot-record/",
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot-record/",
        ] {
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
        }
        "<Ctrl><Shift>R".to_string()
    }

    /// Sets BenderShot video recording shortcut in GNOME
    pub fn set_record_binding(new_binding: &str) -> Result<(), String> {
        let custom_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/bendershot-record/";

        cleanup_legacy_keybindings();
        unbind_conflicting_custom_shortcuts(new_binding, custom_path);

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

        let exe = get_executable_command();
        let cmd = format!("{} record", exe);
        let _ = Command::new("gsettings")
            .args([
                "set",
                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", custom_path),
                "name",
                "BenderShot Gravar Vídeo",
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

pub fn get_executable_command() -> String {
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if !appimage.trim().is_empty() {
            return appimage;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let s = exe.to_string_lossy().to_string();
        if !s.contains(".mount_") {
            return s;
        }
    }
    "bendershot".to_string()
}

fn cleanup_legacy_keybindings() {
    let legacy_paths = [
        "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/",
        "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot-record/",
    ];

    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output()
    {
        if output.status.success() {
            let mut current = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let mut changed = false;

            for legacy in legacy_paths {
                if current.contains(legacy) {
                    let _ = Command::new("gsettings")
                        .args(["reset", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", legacy), "binding"])
                        .output();
                    let _ = Command::new("gsettings")
                        .args(["reset", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", legacy), "command"])
                        .output();
                    let _ = Command::new("gsettings")
                        .args(["reset", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", legacy), "name"])
                        .output();

                    current = current.replace(&format!("'{}', ", legacy), "")
                        .replace(&format!(", '{}'", legacy), "")
                        .replace(&format!("'{}'", legacy), "");
                    changed = true;
                }
            }

            if changed {
                if current.trim() == "[]" || current.trim().is_empty() {
                    current = "@as []".to_string();
                }
                let _ = Command::new("gsettings")
                    .args(["set", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", &current])
                    .output();
            }
        }
    }
}

fn unbind_conflicting_custom_shortcuts(candidate_binding: &str, current_slot: &str) {
    let norm_candidate = normalize_shortcut(candidate_binding);
    if norm_candidate.is_empty() {
        return;
    }

    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for path in text.split('\'').filter(|s| s.starts_with('/')) {
                if path == current_slot {
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
                        let _ = Command::new("gsettings")
                            .args([
                                "reset",
                                &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path),
                                "binding",
                            ])
                            .output();
                    }
                }
            }
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
