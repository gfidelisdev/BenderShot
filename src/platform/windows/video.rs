use std::path::PathBuf;
use std::process::Command;
use anyhow::Result;

pub fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-record.pid")
}

pub fn temp_video_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec.mp4")
}

pub fn video_path_file() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec-path.txt")
}

pub fn stop_flag_file() -> PathBuf {
    std::env::temp_dir().join("bendershot-stop-record.flag")
}

pub fn is_process_running(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::Foundation::CloseHandle;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 as _ {
            false
        } else {
            CloseHandle(handle);
            true
        }
    }
}

pub fn stop_worker_process(_pid: u32) {
    let flag = stop_flag_file();
    let _ = std::fs::write(&flag, b"stop");
}

pub fn send_notification(summary: &str, body: &str, _icon: &str) {
    println!("[BenderShot] {} - {}", summary, body);
}

pub fn detect_screen_dimensions() -> (u32, u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN) as u32;
        if w > 0 && h > 0 {
            return (w, h);
        }
    }
    (1920, 1080)
}

pub async fn run_recorder_worker() -> Result<()> {
    let out_video = temp_video_path();
    let flag = stop_flag_file();
    let _ = std::fs::remove_file(&flag);

    for n in (1..=3).rev() {
        println!("🔴 Gravação começa em {}s...", n);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    println!("Iniciando gravação de tela no Windows...");
    let _ = std::fs::write(video_path_file(), out_video.to_string_lossy().as_bytes());

    // Spawn ffmpeg with gdigrab desktop device
    let ffmpeg = Command::new(crate::video::ScreenRecorder::ffmpeg_bin())
        .args([
            "-y",
            "-f", "gdigrab",
            "-framerate", "30",
            "-i", "desktop",
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-pix_fmt", "yuv420p",
            out_video.to_str().unwrap_or("bendershot-rec.mp4"),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    while !flag.exists() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    if let Ok(mut child) = ffmpeg {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"q\n");
        }
        let _ = child.wait();
    }
    let _ = std::fs::remove_file(&flag);
    Ok(())
}
