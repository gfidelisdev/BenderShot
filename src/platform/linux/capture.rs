use anyhow::{Context, Result};
use ashpd::desktop::screenshot::Screenshot;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use crate::capture::CaptureMode;

/// Pre-authorizes bendershot in the XDG Desktop Portal PermissionStore.
///
/// On Wayland (especially GNOME), when an application triggered via a global
/// shortcut invokes the XDG Desktop Portal screenshot method without interactive
/// prompts, the portal attempts to show an access confirmation dialog. However,
/// GNOME Shell/Mutter blocks background/unfocused processes from showing access dialogs
/// (`Only the focused app is allowed to show a system access dialog`).
///
/// Pre-registering "yes" in `org.freedesktop.impl.portal.PermissionStore` eliminates
/// this prompt and allows instant capture on the very first key press.
pub fn ensure_portal_permissions() {
    let app_ids = ["bendershot", "enhanced-shot", "io.github.whattytheshell", ""];
    for app_id in app_ids {
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.freedesktop.impl.portal.PermissionStore",
                "/org/freedesktop/impl/portal/PermissionStore",
                "org.freedesktop.impl.portal.PermissionStore",
                "SetPermission",
                "sbssas",
                "screenshot",
                "true",
                "screenshot",
                app_id,
                "1",
                "yes",
            ])
            .output();
    }
}

/// Terminates any stale or hung capture/editor processes from previous invocations
pub fn cleanup_stale_capture_processes() {
    let my_pid = std::process::id();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    let pid_file = runtime_dir.join("bendershot-capture.pid");
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if pid != my_pid && is_process_alive(pid) {
                tracing::info!("Terminating previous stale capture process PID {}", pid);
                let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
                std::thread::sleep(Duration::from_millis(50));
                if is_process_alive(pid) {
                    let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).output();
                }
            }
        }
    }

    let _ = std::fs::write(&pid_file, my_pid.to_string());
}

pub fn remove_capture_pid_file() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let pid_file = runtime_dir.join("bendershot-capture.pid");
    let _ = std::fs::remove_file(pid_file);
}

pub fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
    tracing::info!("Triggering Wayland screen capture via XDG Portal (mode: {:?})", mode);

    // Pre-flight: ensure screenshot portal permissions are registered for bendershot
    ensure_portal_permissions();

    // Clean up any stale hanging capture process
    cleanup_stale_capture_processes();

    // Perform portal screenshot with timeout and automatic retry
    let mut last_error = None;
    for attempt in 1..=3 {
        tracing::debug!("Screenshot portal request attempt {}", attempt);

        let request_fut = async {
            let request = Screenshot::request()
                .interactive(false)
                .modal(false);

            let proxy = request
                .send()
                .await
                .context("Failed to send screenshot request to XDG Desktop Portal")?;

            let response = proxy
                .response()
                .context("Empty response from screenshot portal")?;

            Ok::<_, anyhow::Error>(response)
        };

        // 5-second timeout per attempt to guarantee process never hangs indefinitely
        match tokio::time::timeout(Duration::from_secs(5), request_fut).await {
            Ok(Ok(response)) => {
                let uri = response.uri();
                let path = if let Ok(url) = url::Url::parse(uri.as_str()) {
                    url.to_file_path()
                        .map_err(|_| anyhow::anyhow!("Invalid file URI from portal: {}", uri))?
                } else {
                    PathBuf::from(uri.as_str())
                };

                tracing::info!("Screenshot received at path: {:?}", path);
                return load_image_from_path(&path);
            }
            Ok(Err(err)) => {
                tracing::warn!(
                    "Screenshot portal request attempt {} failed: {:?}. Retrying in 150ms...",
                    attempt,
                    err
                );
                last_error = Some(err);
                ensure_portal_permissions();
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            Err(_) => {
                tracing::warn!(
                    "Screenshot portal request attempt {} timed out after 5 seconds. Retrying...",
                    attempt
                );
                last_error = Some(anyhow::anyhow!("Timeout waiting for screenshot portal response"));
                ensure_portal_permissions();
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Falha ao capturar tela após 3 tentativas via XDG Portal")))
}

pub fn load_image_from_path(path: &Path) -> Result<RgbaImage> {
    let dynamic = image::open(path)
        .with_context(|| format!("Failed to open image at {:?}", path))?;
    Ok(dynamic.to_rgba8())
}
