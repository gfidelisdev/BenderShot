use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCapturePayload {
    pub timestamp_str: String,
    pub mode: crate::capture::CaptureMode,
    pub file_path: PathBuf,
    pub delete_file_after_load: bool,
}

#[cfg(not(target_os = "windows"))]
pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    runtime_dir.join("bendershot-editor.sock")
}

#[cfg(target_os = "windows")]
pub fn port_file_path() -> PathBuf {
    std::env::temp_dir().join("bendershot-editor.port")
}

#[cfg(not(target_os = "windows"))]
pub struct IpcServerGuard {
    sock_path: PathBuf,
}

#[cfg(not(target_os = "windows"))]
impl Drop for IpcServerGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.sock_path);
    }
}

#[cfg(target_os = "windows")]
pub struct IpcServerGuard {
    port_path: PathBuf,
}

#[cfg(target_os = "windows")]
impl Drop for IpcServerGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.port_path);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_editor_active() -> bool {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let sock_path = socket_path();
    if !sock_path.exists() {
        return false;
    }

    match UnixStream::connect(&sock_path) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(500)));
            if stream.write_all(b"{\"ping\":true}\n").is_err() {
                let _ = std::fs::remove_file(&sock_path);
                return false;
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.trim() == "PONG" {
                true
            } else {
                let _ = std::fs::remove_file(&sock_path);
                false
            }
        }
        Err(_) => {
            let _ = std::fs::remove_file(&sock_path);
            false
        }
    }
}

#[cfg(target_os = "windows")]
pub fn is_editor_active() -> bool {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let port_file = port_file_path();
    if !port_file.exists() {
        return false;
    }
    let port_str = match std::fs::read_to_string(&port_file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let port: u16 = match port_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            return false;
        }
    };

    let addr = format!("127.0.0.1:{}", port);
    let stream_res = TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        std::time::Duration::from_millis(500),
    );

    match stream_res {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(500)));
            if stream.write_all(b"{\"ping\":true}\n").is_err() {
                let _ = std::fs::remove_file(&port_file);
                return false;
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.trim() == "PONG" {
                true
            } else {
                let _ = std::fs::remove_file(&port_file);
                false
            }
        }
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn try_send_capture(
    image: &image::RgbaImage,
    mode: crate::capture::CaptureMode,
    timestamp_str: &str,
) -> Result<bool> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let sock_path = socket_path();
    if !sock_path.exists() {
        return Ok(false);
    }

    let mut stream = match UnixStream::connect(&sock_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = std::fs::remove_file(&sock_path);
            return Ok(false);
        }
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));

    let tmp_file = std::env::temp_dir().join(format!(
        "bendershot-ipc-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    if let Err(e) = image.save(&tmp_file) {
        return Err(anyhow::anyhow!("Falha ao salvar imagem temporária IPC: {e}"));
    }

    let payload = NewCapturePayload {
        timestamp_str: timestamp_str.to_string(),
        mode,
        file_path: tmp_file.clone(),
        delete_file_after_load: true,
    };

    let json_bytes = serde_json::to_vec(&payload)?;
    if stream.write_all(&json_bytes).is_err() || stream.write_all(b"\n").is_err() {
        let _ = std::fs::remove_file(&tmp_file);
        return Ok(false);
    }
    let _ = stream.flush();

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() && line.trim() == "OK" {
        Ok(true)
    } else {
        let _ = std::fs::remove_file(&tmp_file);
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
pub fn try_send_capture(
    image: &image::RgbaImage,
    mode: crate::capture::CaptureMode,
    timestamp_str: &str,
) -> Result<bool> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let port_file = port_file_path();
    if !port_file.exists() {
        return Ok(false);
    }
    let port_str = match std::fs::read_to_string(&port_file) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let port: u16 = match port_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            return Ok(false);
        }
    };

    let addr = format!("127.0.0.1:{}", port);
    let mut stream = match TcpStream::connect_timeout(
        &addr.parse().map_err(|e| anyhow::anyhow!("{e}"))?,
        std::time::Duration::from_millis(800),
    ) {
        Ok(s) => s,
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            return Ok(false);
        }
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));

    let tmp_file = std::env::temp_dir().join(format!(
        "bendershot-ipc-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    if let Err(e) = image.save(&tmp_file) {
        return Err(anyhow::anyhow!("Falha ao salvar imagem temporária IPC: {e}"));
    }

    let payload = NewCapturePayload {
        timestamp_str: timestamp_str.to_string(),
        mode,
        file_path: tmp_file.clone(),
        delete_file_after_load: true,
    };

    let json_bytes = serde_json::to_vec(&payload)?;
    if stream.write_all(&json_bytes).is_err() || stream.write_all(b"\n").is_err() {
        let _ = std::fs::remove_file(&tmp_file);
        return Ok(false);
    }
    let _ = stream.flush();

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() && line.trim() == "OK" {
        Ok(true)
    } else {
        let _ = std::fs::remove_file(&tmp_file);
        Ok(false)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn try_send_file_path(
    path: &Path,
    mode: crate::capture::CaptureMode,
    timestamp_str: &str,
) -> Result<bool> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let sock_path = socket_path();
    if !sock_path.exists() {
        return Ok(false);
    }

    let mut stream = match UnixStream::connect(&sock_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = std::fs::remove_file(&sock_path);
            return Ok(false);
        }
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));

    let payload = NewCapturePayload {
        timestamp_str: timestamp_str.to_string(),
        mode,
        file_path: path.to_path_buf(),
        delete_file_after_load: false,
    };

    let json_bytes = serde_json::to_vec(&payload)?;
    if stream.write_all(&json_bytes).is_err() || stream.write_all(b"\n").is_err() {
        return Ok(false);
    }
    let _ = stream.flush();

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() && line.trim() == "OK" {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
pub fn try_send_file_path(
    path: &Path,
    mode: crate::capture::CaptureMode,
    timestamp_str: &str,
) -> Result<bool> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let port_file = port_file_path();
    if !port_file.exists() {
        return Ok(false);
    }
    let port_str = match std::fs::read_to_string(&port_file) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let port: u16 = match port_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            return Ok(false);
        }
    };

    let addr = format!("127.0.0.1:{}", port);
    let mut stream = match TcpStream::connect_timeout(
        &addr.parse().map_err(|e| anyhow::anyhow!("{e}"))?,
        std::time::Duration::from_millis(800),
    ) {
        Ok(s) => s,
        Err(_) => {
            let _ = std::fs::remove_file(&port_file);
            return Ok(false);
        }
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));

    let payload = NewCapturePayload {
        timestamp_str: timestamp_str.to_string(),
        mode,
        file_path: path.to_path_buf(),
        delete_file_after_load: false,
    };

    let json_bytes = serde_json::to_vec(&payload)?;
    if stream.write_all(&json_bytes).is_err() || stream.write_all(b"\n").is_err() {
        return Ok(false);
    }
    let _ = stream.flush();

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() && line.trim() == "OK" {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start_editor_ipc_listener(
    tx: std::sync::mpsc::Sender<NewCapturePayload>,
) -> Option<IpcServerGuard> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    let sock_path = socket_path();
    if sock_path.exists() {
        if is_editor_active() {
            tracing::warn!("Another editor instance is already actively listening on socket.");
            return None;
        }
        let _ = std::fs::remove_file(&sock_path);
    }

    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Failed to bind editor IPC socket at {:?}: {}", sock_path, e);
            return None;
        }
    };

    let guard = IpcServerGuard {
        sock_path: sock_path.clone(),
    };

    std::thread::spawn(move || {
        for stream_res in listener.incoming() {
            if let Ok(stream) = stream_res {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));
                let reader_stream = match stream.try_clone() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut reader = BufReader::new(reader_stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let trimmed = line.trim();
                    if trimmed.contains("\"ping\"") {
                        let mut writer = stream;
                        let _ = writer.write_all(b"PONG\n");
                        let _ = writer.flush();
                    } else if let Ok(payload) = serde_json::from_str::<NewCapturePayload>(trimmed) {
                        let _ = tx.send(payload);
                        let mut writer = stream;
                        let _ = writer.write_all(b"OK\n");
                        let _ = writer.flush();
                    }
                }
            }
        }
    });

    Some(guard)
}

#[cfg(target_os = "windows")]
pub fn start_editor_ipc_listener(
    tx: std::sync::mpsc::Sender<NewCapturePayload>,
) -> Option<IpcServerGuard> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let port_file = port_file_path();
    if is_editor_active() {
        tracing::warn!("Another editor instance is already actively listening on port.");
        return None;
    }
    let _ = std::fs::remove_file(&port_file);

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Failed to bind editor IPC TCP listener: {}", e);
            return None;
        }
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            tracing::warn!("Failed to get TCP listener port: {}", e);
            return None;
        }
    };

    if let Err(e) = std::fs::write(&port_file, port.to_string()) {
        tracing::warn!("Failed to write editor IPC port file: {}", e);
        return None;
    }

    let guard = IpcServerGuard {
        port_path: port_file,
    };

    std::thread::spawn(move || {
        for stream_res in listener.incoming() {
            if let Ok(stream) = stream_res {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2000)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(2000)));
                let reader_stream = match stream.try_clone() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut reader = BufReader::new(reader_stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let trimmed = line.trim();
                    if trimmed.contains("\"ping\"") {
                        let mut writer = stream;
                        let _ = writer.write_all(b"PONG\n");
                        let _ = writer.flush();
                    } else if let Ok(payload) = serde_json::from_str::<NewCapturePayload>(trimmed) {
                        let _ = tx.send(payload);
                        let mut writer = stream;
                        let _ = writer.write_all(b"OK\n");
                        let _ = writer.flush();
                    }
                }
            }
        }
    });

    Some(guard)
}
