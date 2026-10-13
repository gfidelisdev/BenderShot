use std::process::Command;
use crate::capture::CaptureMode;

pub fn spawn_capture(mode: CaptureMode) {
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
    tracing::info!("Starting BenderShot background daemon on Windows...");
    println!("BenderShot daemon ativo no Windows. Aguardando comandos ou atalhos.");

    // Keep service running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Exiting Windows indicator daemon.");
    Ok(())
}
