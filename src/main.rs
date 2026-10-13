use anyhow::Result;
use clap::Parser;
use bendershot::cli::{Cli, Commands};
use bendershot::{capture, indicator, ui};
use std::sync::mpsc::channel;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    let command = args.command.unwrap_or(Commands::Region);

    match command {
        Commands::Daemon => {
            println!("BenderShot daemon iniciado. Ícone ativo no painel superior.");
            indicator::run_indicator().await?;
        }
        Commands::Edit { path } => {
            let image = capture::CaptureBackend::load_image_from_path(&path)?;
            open_editor_session(image)?;
        }
        Commands::Region => {
            let image = capture::CaptureBackend::capture(capture::CaptureMode::Region).await?;
            open_editor_session(image)?;
        }
        Commands::Full => {
            let image = capture::CaptureBackend::capture(capture::CaptureMode::Fullscreen).await?;
            open_editor_session(image)?;
        }
        Commands::Window => {
            let image = capture::CaptureBackend::capture(capture::CaptureMode::Window).await?;
            open_editor_session(image)?;
        }
        Commands::Record => {
            if bendershot::video::ScreenRecorder::is_recording() {
                println!("Finalizando gravação de tela...");
                let video_path = bendershot::video::ScreenRecorder::stop_recording()?;
                println!("Gravação concluída. Abrindo editor de vídeo para: {:?}", video_path);
                bendershot::ui::editor::run_video_editor(video_path)
                    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            } else {
                bendershot::video::ScreenRecorder::start_recording()?;
            }
        }
        Commands::RecordWorker => {
            bendershot::video::ScreenRecorder::run_worker().await?;
        }
    }

    Ok(())
}

fn open_editor_session(image: image::RgbaImage) -> Result<()> {
    let (pin_tx, pin_rx) = channel::<image::RgbaImage>();

    // Run the main editor window
    let _ = ui::run_editor(image, Some(pin_tx));

    // If an image was sent to be pinned, open the floating pin window
    if let Ok(pinned_image) = pin_rx.try_recv() {
        let _ = ui::run_pin_window(pinned_image);
    }

    Ok(())
}
