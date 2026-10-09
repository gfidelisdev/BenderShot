use crate::capture::CaptureMode;
use ksni::{MenuItem, Tray, TrayMethods};
use std::process::Command;

#[derive(Debug, Default)]
pub struct EnhancedShotTray;

impl Tray for EnhancedShotTray {
    fn id(&self) -> String {
        "enhanced-shot".into()
    }

    fn title(&self) -> String {
        "Enhanced Shot".into()
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
    if let Ok(exe) = std::env::current_exe() {
        let arg = match mode {
            CaptureMode::Region => "region",
            CaptureMode::Fullscreen => "full",
            CaptureMode::Window => "window",
        };
        let _ = Command::new(exe).arg(arg).spawn();
    }
}

pub async fn run_indicator() -> anyhow::Result<()> {
    tracing::info!("Starting Enhanced Shot top-panel indicator service...");
    let _handle = EnhancedShotTray.spawn().await?;

    // Keep service running in Tokio runtime
    tokio::signal::ctrl_c().await?;
    tracing::info!("Exiting indicator service.");
    Ok(())
}
