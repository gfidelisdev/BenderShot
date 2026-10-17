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

const GNOME_HELPER: &str = include_str!("gnome_helper.py");

pub async fn capture(mode: CaptureMode) -> Result<RgbaImage> {
    tracing::info!("Triggering Wayland screen capture (mode: {:?})", mode);

    // Clean up any stale hanging capture process
    cleanup_stale_capture_processes();

    if mode == CaptureMode::Window {
        match capture_focused_window().await {
            Ok(img) => return Ok(img),
            Err(e) => tracing::warn!("Focused-window capture failed ({e:?}); falling back to full desktop"),
        }
    }

    portal_capture_desktop().await
}

/// Opens the live-screen transparent region selector overlay across all monitors.
/// Returns `Ok(Some(selection))` when the user clicks the tick button, or `Ok(None)` if cancelled.
pub fn select_region_live() -> Result<Option<crate::capture::RegionSelection>> {
    cleanup_stale_capture_processes();
    let output = Command::new("python3")
        .arg("-W")
        .arg("ignore")
        .arg("-c")
        .arg(GNOME_HELPER)
        .arg("select")
        .output()
        .context("Failed to launch live region selector helper")?;
    if !output.status.success() {
        anyhow::bail!(
            "Live region selector exited with error: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let sel: Option<crate::capture::RegionSelection> = serde_json::from_slice(&output.stdout)
        .context("Failed to parse region selector output")?;
    Ok(sel)
}

/// Captures the full desktop after the live-screen overlay closed and crops the user's selection.
pub async fn capture_region(sel: crate::capture::RegionSelection) -> Result<RgbaImage> {
    cleanup_stale_capture_processes();
    let desktop = portal_capture_desktop().await?;
    let monitors = query_monitor_layout().await.unwrap_or_else(|e| {
        tracing::warn!("Monitor layout query failed ({e:?}); assuming single monitor");
        Vec::new()
    });
    crate::capture::crop_selection_from_desktop(&desktop, &monitors, &sel)
        .context("Seleção de região vazia ou fora da tela")
}

/// Captures the focused window through org.gnome.Mutter.ScreenCast (RecordWindow without a
/// window-id targets the focused window). The window stream buffer can be larger than the
/// window, so transparent borders are trimmed.
async fn capture_focused_window() -> Result<RgbaImage> {
    let out = std::env::temp_dir().join(format!("bendershot-window-{}.png", std::process::id()));
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("python3")
            .arg("-c")
            .arg(GNOME_HELPER)
            .arg("window")
            .arg(&out)
            .output(),
    )
    .await
    .context("Timeout capturing focused window")?
    .context("Failed to run GNOME window capture helper")?;
    if !output.status.success() {
        anyhow::bail!("Window helper failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let img = load_image_from_path(&out);
    let _ = std::fs::remove_file(&out);
    Ok(crate::capture::trim_transparent(img?))
}

async fn query_monitor_layout() -> Result<Vec<crate::capture::MonitorLayout>> {
    let output = tokio::process::Command::new("python3")
        .arg("-c")
        .arg(GNOME_HELPER)
        .arg("monitors")
        .output()
        .await
        .context("Failed to run GNOME monitor helper")?;
    if !output.status.success() {
        anyhow::bail!("Monitor helper failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Silent (non-interactive) XDG portal screenshot of the whole desktop, with retries.
async fn portal_capture_desktop() -> Result<RgbaImage> {
    // Pre-flight: ensure screenshot portal permissions are registered for bendershot
    ensure_portal_permissions();

    // Perform portal screenshot with timeout and automatic retry
    let mut last_error = None;
    let interactive = false;
    let attempts = 3;
    let timeout_duration = Duration::from_secs(5);

    for attempt in 1..=attempts {
        tracing::debug!("Screenshot portal request attempt {}", attempt);

        let request_fut = async {
            let request = Screenshot::request()
                .interactive(interactive)
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

        // Timeout per attempt to guarantee process never hangs indefinitely
        match tokio::time::timeout(timeout_duration, request_fut).await {
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
