use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use image::RgbaImage;

pub struct ScreenRecorder;

impl ScreenRecorder {
    /// Checks if a recording session is currently active
    pub fn is_recording() -> bool {
        let pid_path = crate::platform::imp::video::pid_file_path();
        if let Ok(content) = std::fs::read_to_string(&pid_path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                return crate::platform::imp::video::is_process_running(pid);
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
            .unwrap_or_else(|_| PathBuf::from("bendershot"));

        let mut cmd = Command::new(exe);
        cmd.arg("record-worker")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let child = cmd.spawn().context("Falha ao iniciar processo de gravação")?;
        let pid = child.id();

        // Write PID file
        let pid_path = crate::platform::imp::video::pid_file_path();
        std::fs::write(&pid_path, pid.to_string())
            .context("Falha ao escrever arquivo PID de gravação")?;

        // Give worker a brief moment to initialize
        std::thread::sleep(std::time::Duration::from_millis(300));
        Ok(())
    }

    /// Runs the persistent recording worker maintaining the screencast session
    pub async fn run_worker() -> Result<()> {
        crate::platform::imp::video::run_recorder_worker().await
    }

    /// Stops active recording session and returns recorded file path
    pub fn stop_recording() -> Result<PathBuf> {
        let pid_path = crate::platform::imp::video::pid_file_path();
        if let Ok(content) = std::fs::read_to_string(&pid_path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                crate::platform::imp::video::stop_worker_process(pid);

                // Wait up to 15s for encoder finalization
                for _ in 0..150 {
                    if !crate::platform::imp::video::is_process_running(pid) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        let _ = std::fs::remove_file(&pid_path);

        // Notify user that recording completed
        crate::platform::imp::video::send_notification(
            "🎬 Gravação Concluída",
            "Abrindo editor de vídeo...",
            "video-x-generic",
        );

        let path_file = crate::platform::imp::video::video_path_file();
        if let Ok(path_str) = std::fs::read_to_string(&path_file) {
            let p = PathBuf::from(path_str.trim());
            if p.exists() {
                let _ = std::fs::remove_file(&path_file);
                return Ok(p);
            }
        }

        // Fallback default temp path
        Ok(crate::platform::imp::video::temp_video_path())
    }

    /// Detects desktop resolution across all monitors
    pub fn detect_screen_dimensions() -> (u32, u32) {
        crate::platform::imp::video::detect_screen_dimensions()
    }

    /// Detects best available video encoder in ffmpeg
    pub fn detect_video_encoder() -> String {
        if let Ok(output) = Command::new(Self::ffmpeg_bin()).args(["-encoders"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("libx264") {
                return "libx264".into();
            }
            if text.contains("libopenh264") {
                return "libopenh264".into();
            }
            if text.contains("h264_nvenc") {
                return "h264_nvenc".into();
            }
            if text.contains("h264_vaapi") {
                return "h264_vaapi".into();
            }
            if text.contains("mpeg4") {
                return "mpeg4".into();
            }
        }
        "libx264".into()
    }

    /// Locates the ffmpeg binary (local to app directory or in PATH)
    pub fn ffmpeg_bin() -> std::ffi::OsString {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let local = dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
                if local.exists() {
                    return local.into_os_string();
                }
            }
        }
        (if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }).into()
    }

    /// Locates the ffprobe binary (local to app directory or in PATH)
    pub fn ffprobe_bin() -> std::ffi::OsString {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let local = dir.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
                if local.exists() {
                    return local.into_os_string();
                }
            }
        }
        (if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" }).into()
    }
}

pub struct VideoExporter;

impl VideoExporter {
    /// Probes video duration in milliseconds via ffprobe
    pub fn probe_video_duration(video_path: &Path) -> Result<u64> {
        let out = Command::new(ScreenRecorder::ffprobe_bin())
            .args([
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                video_path.to_str().unwrap_or_default(),
            ])
            .output()?;

        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(sec) = text.parse::<f64>() {
                return Ok((sec * 1000.0) as u64);
            }
        }
        Ok(5000)
    }

    /// Extracts video frame at specific timestamp in ms as RgbaImage
    pub fn extract_frame_at(video_path: &Path, time_ms: u64) -> Result<RgbaImage> {
        let sec = (time_ms as f64) / 1000.0;
        let tmp_frame = std::env::temp_dir().join(format!("bendershot-frame-{}.png", std::process::id()));

        let out = Command::new(ScreenRecorder::ffmpeg_bin())
            .args([
                "-y",
                "-ss", &format!("{:.3}", sec),
                "-i", video_path.to_str().unwrap_or_default(),
                "-vframes", "1",
                "-q:v", "2",
                tmp_frame.to_str().unwrap_or_default(),
            ])
            .output()?;

        if out.status.success() && tmp_frame.exists() {
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
        scene: Option<&crate::model::Scene>,
    ) -> Result<()> {
        let encoder = ScreenRecorder::detect_video_encoder();

        if let Some(sc) = scene {
            if !sc.items.is_empty() {
                // Burn annotations onto frames
                let tmp_dir = std::env::temp_dir().join(format!("bendershot-export-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&tmp_dir);
                std::fs::create_dir_all(&tmp_dir)?;

                let fps = 15;
                let pattern = tmp_dir.join("f%05d.png");
                let _ = Command::new("ffmpeg")
                    .args([
                        "-y", "-loglevel", "error",
                        "-i", input_path.to_str().unwrap_or_default(),
                        "-vf", &format!("fps={}", fps),
                        pattern.to_str().unwrap_or_default(),
                    ])
                    .output()?;

                let mut frame_idx = 1;
                while let Ok(img) = image::open(tmp_dir.join(format!("f{:05}.png", frame_idx))) {
                    let time_ms = (frame_idx - 1) as u64 * 1000 / fps as u64;
                    let rgba = img.to_rgba8();
                    let rendered = crate::rasterize::rasterize_scene_at_time(&rgba, sc, time_ms);
                    rendered.save(tmp_dir.join(format!("f{:05}.png", frame_idx)))?;
                    frame_idx += 1;
                }

                if frame_idx > 1 {
                    let out = Command::new(ScreenRecorder::ffmpeg_bin())
                        .args([
                            "-y",
                            "-framerate", &fps.to_string(),
                            "-i", pattern.to_str().unwrap_or_default(),
                            "-c:v", &encoder,
                            "-pix_fmt", "yuv420p",
                            output_path.to_str().unwrap_or_default(),
                        ])
                        .output()?;
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    if out.status.success() {
                        return Ok(());
                    }
                }
            }
        }

        // Fast fallback or copy when no annotations
        let out = Command::new(ScreenRecorder::ffmpeg_bin())
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

        // Fallback: try stream copy
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
