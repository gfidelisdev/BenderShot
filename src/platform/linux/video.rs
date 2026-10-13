use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};

pub fn pid_file_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-record.pid")
}

pub fn temp_video_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec.mp4")
}

pub fn video_path_file() -> PathBuf {
    std::env::temp_dir().join("bendershot-rec-path.txt")
}

pub fn is_process_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

pub fn stop_worker_process(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
}

pub fn send_notification(summary: &str, body: &str, icon: &str) {
    let _ = Command::new("notify-send")
        .args(["-i", icon, "-a", "BenderShot", summary, body])
        .status();
}

pub fn detect_screen_dimensions() -> (u32, u32) {
    if let Ok(output) = Command::new("xdpyinfo").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("dimensions:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let dims: Vec<&str> = parts[1].split('x').collect();
                        if dims.len() == 2 {
                            if let (Ok(w), Ok(h)) = (dims[0].parse::<u32>(), dims[1].parse::<u32>()) {
                                return (w, h);
                            }
                        }
                    }
                }
            }
        }
    }
    (1920, 1080)
}

pub async fn run_recorder_worker() -> Result<()> {
    // 1. Countdown banner
    let mut notif_id: Option<String> = None;
    for n in (1..=3).rev() {
        let mut cmd = Command::new("notify-send");
        cmd.args(["-p", "-i", "media-record", "-a", "BenderShot", "-t", "1500"]);
        if let Some(ref id) = notif_id {
            cmd.args(["-r", id]);
        }
        cmd.args([
            &format!("🔴 Gravação começa em {}…", n),
            "Pressione o atalho novamente para finalizar.",
        ]);
        if let Ok(out) = cmd.output() {
            let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !id.is_empty() {
                notif_id = Some(id);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    if let Some(ref id) = notif_id {
        let _ = Command::new("gdbus")
            .args([
                "call", "--session",
                "--dest", "org.freedesktop.Notifications",
                "--object-path", "/org/freedesktop/Notifications",
                "--method", "org.freedesktop.Notifications.CloseNotification",
                id,
            ])
            .output();
    }
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 2. Start GNOME Screencast via python helper script
    let path_file = video_path_file();
    let script = format!(r#"
import dbus, sys, time, signal

bus = dbus.SessionBus()
proxy = bus.get_object("org.gnome.Shell.Screencast", "/org/gnome/Shell/Screencast")
iface = dbus.Interface(proxy, "org.gnome.Shell.Screencast")

res = iface.Screencast("bendershot_rec_%d_%t", {{}})
filename = str(res[1])

with open({:?}, "w") as f:
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
"#, path_file.to_string_lossy());

    let mut py_cmd = Command::new("python3");
    py_cmd.args(["-c", &script])
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
