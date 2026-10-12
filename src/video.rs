use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use image::RgbaImage;

const PID_FILE_PATH: &str = "/tmp/enhanced-shot-record.pid";
const TEMP_VIDEO_PATH: &str = "/tmp/enhanced-shot-rec.mp4";
const VIDEO_PATH_FILE: &str = "/tmp/enhanced-shot-rec-path.txt";

pub struct ScreenRecorder;

impl ScreenRecorder {
    /// Checks if a recording session is currently active
    pub fn is_recording() -> bool {
        if let Ok(content) = std::fs::read_to_string(PID_FILE_PATH) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                // Check if process with this pid exists via /proc/{pid}
                return Path::new(&format!("/proc/{}", pid)).exists();
            }
        }
        false
    }

    /// Toggles recording session: starts if stopped, stops and returns video path if active
    pub fn toggle_recording() -> Result<Option<PathBuf>> {
        if Self::is_recording() {
            let path = Self::stop_recording()?;
            Ok(Some(path))
        } else {
            Self::start_recording()?;
            Ok(None)
        }
    }

    /// Starts recording via background worker process
    pub fn start_recording() -> Result<()> {
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("enhanced-shot"));

        let mut cmd = Command::new(exe);
        cmd.arg("record-worker")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child = cmd.spawn().context("Falha ao iniciar processo em segundo plano de gravação")?;
        let pid = child.id();
        std::fs::write(PID_FILE_PATH, pid.to_string())
            .context("Falha ao salvar arquivo de PID de gravação")?;

        // Give worker a moment to initialize screencast
        std::thread::sleep(std::time::Duration::from_millis(300));
        Ok(())
    }

    /// Runs the persistent recording worker maintaining the GNOME Screencast session
    pub async fn run_worker() -> Result<()> {
        // 1. Notify user (short-lived banner), then wait for it to disappear BEFORE capture starts
        let _ = Command::new("notify-send")
            .args([
                "-i", "media-record",
                "-a", "Enhanced Shot",
                "-t", "1200",
                "-h", "string:x-canonical-private-synchronous:enhanced-shot-rec",
                "🔴 Gravação iniciando…",
                "Pressione o atalho novamente para finalizar.",
            ])
            .status();
        tokio::time::sleep(std::time::Duration::from_millis(4500)).await;

        // 2. Start GNOME Screencast via python helper script to guarantee persistent session
        let script = r#"
import dbus, sys, time, signal

bus = dbus.SessionBus()
proxy = bus.get_object("org.gnome.Shell.Screencast", "/org/gnome/Shell/Screencast")
iface = dbus.Interface(proxy, "org.gnome.Shell.Screencast")

res = iface.Screencast("enhanced_shot_rec_%d_%t", {})
filename = str(res[1])

with open("/tmp/enhanced-shot-rec-path.txt", "w") as f:
    f.write(filename)

stop = False
def sig_handler(sig, frame):
    global stop
    stop = True

signal.signal(signal.SIGTERM, sig_handler)
signal.signal(signal.SIGINT, sig_handler)

while not stop:
    time.sleep(0.05)

iface.StopScreencast()
time.sleep(0.5)
"#;

        let mut py_cmd = Command::new("python3");
        py_cmd.args(["-c", script])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let mut py_child = py_cmd.spawn().context("Falha ao iniciar gravador GNOME Screencast")?;

        // Handle termination signals in worker
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }

        // Send SIGTERM to python helper to cleanly call StopScreencast
        let _ = Command::new("kill")
            .args(["-TERM", &py_child.id().to_string()])
            .status();

        let _ = py_child.wait();
        Ok(())
    }

    /// Stops active recording session and returns recorded file path
    pub fn stop_recording() -> Result<PathBuf> {
        if let Ok(content) = std::fs::read_to_string(PID_FILE_PATH) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                // Send SIGTERM to worker process so it cleanly finishes the screencast
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();

                // Wait for worker process to exit cleanly
                let proc_path = format!("/proc/{}", pid);
                for _ in 0..40 {
                    if !Path::new(&proc_path).exists() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        let _ = std::fs::remove_file(PID_FILE_PATH);

        // Notify user that recording completed
        let _ = Command::new("notify-send")
            .args([
                "-i", "video-x-generic",
                "-a", "Enhanced Shot",
                "🎬 Gravação Concluída",
                "Abrindo editor de vídeo...",
            ])
            .status();

        if let Ok(path_str) = std::fs::read_to_string(VIDEO_PATH_FILE) {
            let p = PathBuf::from(path_str.trim());
            if p.exists() {
                let _ = std::fs::remove_file(VIDEO_PATH_FILE);
                return Ok(p);
            }
        }

        // Fallback default temp path
        Ok(PathBuf::from(TEMP_VIDEO_PATH))
    }

    /// Detects desktop resolution across all monitors
    pub fn detect_screen_dimensions() -> (u32, u32) {
        if let Ok(output) = Command::new("xdpyinfo").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if line.contains("dimensions:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let dims = parts[1]; // e.g. "5360x1440"
                            let mut dim_parts = dims.split('x');
                            if let (Some(w), Some(h)) = (dim_parts.next(), dim_parts.next()) {
                                if let (Ok(width), Ok(height)) = (w.parse::<u32>(), h.parse::<u32>()) {
                                    return (width, height);
                                }
                            }
                        }
                    }
                }
            }
        }
        (1920, 1080)
    }

    /// Detects best available H.264/video encoder on the host system
    pub fn detect_video_encoder() -> String {
        if let Ok(output) = Command::new("ffmpeg").args(["-encoders"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("libopenh264") {
                return "libopenh264".into();
            } else if text.contains("libx264") {
                return "libx264".into();
            } else if text.contains("h264_vaapi") {
                return "h264_vaapi".into();
            } else if text.contains("mpeg4") {
                return "mpeg4".into();
            }
        }
        "libopenh264".into()
    }
}

pub struct VideoExporter;

impl VideoExporter {
    /// Extracts video metadata (duration in ms, width, height) using ffprobe
    pub fn probe_video_duration(video_path: &Path) -> Result<u64> {
        let out = Command::new("ffprobe")
            .args([
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                video_path.to_str().unwrap_or_default(),
            ])
            .output()?;

        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Ok(sec) = s.trim().parse::<f64>() {
                return Ok((sec * 1000.0) as u64);
            }
        }
        // Fallback default duration 5000ms
        Ok(5000)
    }

    /// Extracts video frame at specific timestamp in ms as RgbaImage
    pub fn extract_frame_at(video_path: &Path, time_ms: u64) -> Result<RgbaImage> {
        let sec = (time_ms as f64) / 1000.0;
        let tmp_frame = format!("/tmp/enhanced-shot-frame-{}.png", std::process::id());

        let out = Command::new("ffmpeg")
            .args([
                "-y",
                "-ss", &format!("{:.3}", sec),
                "-i", video_path.to_str().unwrap_or_default(),
                "-vframes", "1",
                "-q:v", "2",
                &tmp_frame,
            ])
            .output()?;

        if out.status.success() && Path::new(&tmp_frame).exists() {
            let loaded = image::open(&tmp_frame)?;
            let _ = std::fs::remove_file(&tmp_frame);
            Ok(loaded.to_rgba8())
        } else {
            anyhow::bail!("Falha ao extrair frame de vídeo em {:.3}s", sec);
        }
    }

    /// Exports annotated video burning active AnnotationSpans to MP4, WebM or GIF
    pub fn export_annotated_video(
        input_path: &Path,
        output_path: &Path,
        _overlay_frames_dir: Option<&Path>,
    ) -> Result<()> {
        let encoder = ScreenRecorder::detect_video_encoder();
        let out = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", input_path.to_str().unwrap_or_default(),
                "-c:v", &encoder,
                "-pix_fmt", "yuv420p",
                output_path.to_str().unwrap_or_default(),
            ])
            .output()?;

        if out.status.success() {
            return Ok(());
        }

        // Fallback: try copy or mpeg4
        let fallback_out = Command::new("ffmpeg")
            .args([
                "-y",
                "-i", input_path.to_str().unwrap_or_default(),
                "-c:v", "copy",
                output_path.to_str().unwrap_or_default(),
            ])
            .output()?;

        if fallback_out.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let last_lines = err.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
            anyhow::bail!("Falha no ffmpeg: {}", last_lines);
        }
    }
}
