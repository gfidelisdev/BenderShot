use crate::capture::CaptureMode;
use ksni::{MenuItem, Tray, TrayMethods};
use std::process::Command;

#[derive(Debug, Default)]
pub struct BenderShotTray;

impl Tray for BenderShotTray {
    fn id(&self) -> String {
        "bendershot".into()
    }

    fn title(&self) -> String {
        "BenderShot".into()
    }

    fn icon_name(&self) -> String {
        "applets-screenshooter".into()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;

        vec![
            StandardItem {
                label: "📸 Capturar Região".into(),
                activate: Box::new(|_| {
                    spawn_capture(CaptureMode::Region);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "🖥️ Capturar Tela Cheia".into(),
                activate: Box::new(|_| {
                    spawn_capture(CaptureMode::Fullscreen);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "🪟 Capturar Janela".into(),
                activate: Box::new(|_| {
                    spawn_capture(CaptureMode::Window);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "🎬 Gravar Tela (Vídeo)".into(),
                activate: Box::new(|_| {
                    spawn_record();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "🧙 Assistente de Configuração".into(),
                activate: Box::new(|_| {
                    spawn_wizard();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "⚙️ Configurações".into(),
                activate: Box::new(|_| {
                    spawn_settings();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "❌ Sair".into(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn spawn_capture(mode: CaptureMode) {
    let exe = crate::platform::linux::shortcuts::get_executable_command();
    let arg = match mode {
        CaptureMode::Region => "region",
        CaptureMode::Fullscreen => "full",
        CaptureMode::Window => "window",
    };
    let _ = Command::new(exe).arg(arg).spawn();
}

fn spawn_record() {
    let exe = crate::platform::linux::shortcuts::get_executable_command();
    let _ = Command::new(exe).arg("record").spawn();
}

fn spawn_settings() {
    let exe = crate::platform::linux::shortcuts::get_executable_command();
    let _ = Command::new(exe).arg("settings").spawn();
}

fn spawn_wizard() {
    let exe = crate::platform::linux::shortcuts::get_executable_command();
    let _ = Command::new(exe).arg("wizard").spawn();
}

pub async fn run_indicator() -> anyhow::Result<()> {
    tracing::info!("Starting BenderShot top-panel indicator service...");

    // Single-instance check for daemon
    let my_pid = std::process::id();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let daemon_pid_file = runtime_dir.join("bendershot-daemon.pid");

    if let Ok(content) = std::fs::read_to_string(&daemon_pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if pid != my_pid && crate::platform::linux::capture::is_process_alive(pid) {
                tracing::info!(
                    "BenderShot daemon já está ativo com PID {}. Encerrando instância redundante.",
                    pid
                );
                return Ok(());
            }
        }
    }
    let _ = std::fs::write(&daemon_pid_file, my_pid.to_string());

    // Pre-authorize XDG Portal screenshot permissions
    crate::platform::linux::capture::ensure_portal_permissions();

    // Auto-configure shortcuts on first run of daemon if not already completed
    if !crate::config::AppConfig::is_setup_completed() {
        tracing::info!("First run of BenderShot daemon detected. Auto-configuring GNOME shortcuts...");
        let _ = crate::shortcuts::ShortcutManager::set_binding("Print");
        let _ = crate::shortcuts::ShortcutManager::set_record_binding("<Ctrl><Shift>R");
        let _ = crate::shortcuts::ShortcutManager::configure_autostart(true);
        crate::config::AppConfig::mark_setup_completed();
        crate::shortcuts::ShortcutManager::send_notification(
            "BenderShot Ativo",
            "BenderShot está ativo na bandeja! Pressione PrintScreen para capturar a tela.",
        );
    }

    match BenderShotTray.spawn().await {
        Ok(_handle) => {
            tracing::info!("BenderShot tray indicator active.");
            tokio::signal::ctrl_c().await?;
        }
        Err(e) => {
            tracing::warn!(
                "StatusNotifierWatcher não disponível no ambiente gráfico atual (ex: GNOME padrão sem extensão AppIndicator): {}. Mantendo serviço daemon ativo em segundo plano.",
                e
            );

            // Keep service alive and periodically attempt to connect if tray service starts
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Ok(_handle) = BenderShotTray.spawn().await {
                            tracing::info!("Conectado com sucesso ao StatusNotifierWatcher (bandeja ativada)!");
                            let _ = tokio::signal::ctrl_c().await;
                            break;
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("Exiting indicator service.");
    let _ = std::fs::remove_file(daemon_pid_file);
    Ok(())
}
