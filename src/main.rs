#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;
use bendershot::cli::{Cli, Commands};
use bendershot::{capture, indicator, ui};
use std::sync::mpsc::channel;

#[tokio::main]
async fn main() -> Result<()> {
    let res = real_main().await;
    if let Err(e) = &res {
        report_fatal_error(&format!("{e:#}"));
    }
    res
}

/// With `windows_subsystem = "windows"` there is no console, so failures would be invisible.
/// Persist them to a log file and show a message box on Windows.
fn report_fatal_error(msg: &str) {
    eprintln!("Erro: {msg}");
    #[cfg(target_os = "windows")]
    {
        let log = std::env::temp_dir().join("bendershot-error.log");
        let _ = std::fs::write(&log, msg);
        let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
        let text = wide(&format!("{msg}\n\nLog: {}", log.display()));
        let title = wide("BenderShot");
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                0 as _,
                text.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}

async fn real_main() -> Result<()> {
    #[cfg(target_os = "windows")]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
        windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }

    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    let command = match args.command {
        Some(cmd) => cmd,
        None => {
            if !bendershot::config::AppConfig::is_setup_completed() {
                Commands::Wizard
            } else {
                Commands::Region
            }
        }
    };

    match command {
        Commands::Daemon => {
            #[cfg(target_os = "linux")]
            println!("BenderShot daemon iniciado. Ícone ativo no painel superior.");
            indicator::run_indicator().await?;
        }
        Commands::Wizard => {
            ui::run_wizard_window().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        }
        Commands::AutoSetup => {
            println!("Executando configuração automática do BenderShot...");
            #[cfg(target_os = "linux")]
            bendershot::platform::linux::capture::ensure_portal_permissions();

            let mut ok = true;
            let default_capture = if cfg!(target_os = "windows") {
                "Ctrl+Print"
            } else {
                "Print"
            };
            if let Err(e) = bendershot::shortcuts::ShortcutManager::set_binding(default_capture) {
                eprintln!("Aviso: Falha ao configurar atalho de captura: {}", e);
                ok = false;
            } else {
                println!("✓ Atalho de captura vinculado ({}) ao BenderShot.", default_capture);
            }
            if let Err(e) = bendershot::shortcuts::ShortcutManager::set_window_binding("<Alt>Print") {
                eprintln!("Aviso: Falha ao configurar atalho de janela: {}", e);
            } else {
                println!("✓ Atalho de captura de janela <Alt>Print configurado.");
            }
            if let Err(e) = bendershot::shortcuts::ShortcutManager::set_region_binding("<Shift>Print") {
                eprintln!("Aviso: Falha ao configurar atalho de trecho: {}", e);
            } else {
                println!("✓ Atalho de captura de trecho <Shift>Print configurado.");
            }
            if let Err(e) = bendershot::shortcuts::ShortcutManager::set_record_binding("<Ctrl><Shift>R") {
                eprintln!("Aviso: Falha ao configurar atalho de gravação: {}", e);
            } else {
                println!("✓ Atalho de vídeo <Ctrl><Shift>R configurado.");
            }
            if let Err(e) = bendershot::shortcuts::ShortcutManager::configure_autostart(true) {
                eprintln!("Aviso: Falha ao configurar autostart: {}", e);
            } else {
                println!("✓ Inicialização automática (autostart) configurada.");
            }
            if bendershot::shortcuts::ShortcutManager::is_running_as_appimage() {
                if let Err(e) = bendershot::shortcuts::ShortcutManager::integrate_appimage() {
                    eprintln!("Aviso: Falha na integração do AppImage: {}", e);
                } else {
                    println!("✓ AppImage integrado ao menu do sistema.");
                }
            }
            if let Err(e) = bendershot::shortcuts::ShortcutManager::start_daemon_if_not_running() {
                eprintln!("Aviso: Falha ao iniciar daemon: {}", e);
            } else {
                println!("✓ Daemon do BenderShot iniciado.");
            }
            bendershot::config::AppConfig::mark_setup_completed();
            bendershot::shortcuts::ShortcutManager::send_notification(
                "BenderShot Ativo",
                "Configuração concluída! Pressione PrintScreen para capturar a tela."
            );
            if ok {
                println!("✓ BenderShot configurado e pronto para uso!");
            }
        }
        Commands::Edit { path } => {
            let image = capture::CaptureBackend::load_image_from_path(&path)?;
            open_editor_session(image, capture::CaptureMode::Fullscreen)?;
        }
        Commands::Region => {
            if let Some(selection) = ui::run_region_selector().map_err(|e| anyhow::anyhow!("{e:?}"))? {
                // Allow the compositor to unmap the transparent overlay before capturing
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                let cropped = capture::CaptureBackend::capture_region(selection).await?;
                #[cfg(target_os = "windows")]
                {
                    // winit cannot reliably reuse an event loop after the transparent overlay
                    // closed, so the editor is opened in a fresh process.
                    let tmp = std::env::temp_dir().join(format!("bendershot-region-{}.png", std::process::id()));
                    cropped.save(&tmp)?;
                    use std::os::windows::process::CommandExt;
                    std::process::Command::new(std::env::current_exe()?)
                        .arg("edit")
                        .arg(&tmp)
                        .creation_flags(0x08000000)
                        .spawn()?;
                    std::process::exit(0);
                }
                #[cfg(not(target_os = "windows"))]
                open_editor_session(cropped, capture::CaptureMode::Fullscreen)?;
            } else {
                #[cfg(target_os = "linux")]
                bendershot::platform::linux::capture::remove_capture_pid_file();
                #[cfg(target_os = "windows")]
                std::process::exit(0);
            }
        }
        Commands::Full => {
            let image = capture::CaptureBackend::capture(capture::CaptureMode::Fullscreen).await?;
            open_editor_session(image, capture::CaptureMode::Fullscreen)?;
        }
        Commands::Window => {
            let image = capture::CaptureBackend::capture(capture::CaptureMode::Window).await?;
            open_editor_session(image, capture::CaptureMode::Window)?;
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
        Commands::Settings => {
            ui::run_settings_window().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        }
        Commands::Update { check, gui, yes } => {
            if gui {
                ui::run_updater_window().map_err(|e| anyhow::anyhow!("{e:?}"))?;
                return Ok(());
            }

            let current_ver = bendershot::config::AppConfig::get_effective_version();
            let method = bendershot::updater::detect_install_method();
            println!("==> BenderShot Updater");
            println!("Versão instalada: {}", current_ver);
            println!("Método de instalação: {}", method);
            println!("Consultando lançamentos no GitHub...");

            match bendershot::updater::check_for_updates() {
                Ok(Some(release)) => {
                    println!("\n✨ Nova versão disponível: {} (atual: {})", release.tag_name, current_ver);
                    if let Some(body) = &release.body {
                        println!("\n--- Notas de Versão ---");
                        println!("{}", body.trim());
                        println!("-----------------------");
                    }

                    if check {
                        println!("\nPara atualizar, execute: bendershot update");
                        return Ok(());
                    }

                    let should_proceed = if yes {
                        true
                    } else {
                        print!("\nDeseja instalar a versão {} agora? [S/n]: ", release.tag_name);
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        let ans = input.trim().to_lowercase();
                        ans.is_empty() || ans == "s" || ans == "sim" || ans == "y" || ans == "yes"
                    };

                    if should_proceed {
                        println!("\nIniciando atualização...");
                        bendershot::updater::perform_in_place_update(&release, |msg| {
                            println!("  [i] {}", msg);
                        })
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                        println!("\n✔ Atualização para {} concluída com sucesso!", release.tag_name);
                    } else {
                        println!("Atualização cancelada pelo usuário.");
                    }
                }
                Ok(None) => {
                    println!("✔ O BenderShot já está na versão mais recente ({}).", current_ver);
                }
                Err(e) => {
                    eprintln!("Erro ao verificar atualizações: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn open_editor_session(image: image::RgbaImage, mode: capture::CaptureMode) -> Result<()> {
    let (pin_tx, pin_rx) = channel::<image::RgbaImage>();

    // Run the main editor window
    if let Err(e) = ui::run_editor(image, Some(pin_tx), mode) {
        return Err(anyhow::anyhow!("Falha ao abrir o editor: {e:?}"));
    }

    // If an image was sent to be pinned, open the floating pin window
    if let Ok(pinned_image) = pin_rx.try_recv() {
        let _ = ui::run_pin_window(pinned_image);
    }

    #[cfg(target_os = "linux")]
    bendershot::platform::linux::capture::remove_capture_pid_file();

    Ok(())
}
