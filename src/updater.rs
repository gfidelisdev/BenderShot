use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GITHUB_REPO: &str = "gfidelisdev/BenderShot";
pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/gfidelisdev/BenderShot/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallMethod {
    AppImage(PathBuf),
    SystemDeb,
    SystemRpm,
    StandaloneBinary(PathBuf),
    WindowsInstaller,
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::AppImage(p) => write!(f, "AppImage ({})", p.display()),
            InstallMethod::SystemDeb => write!(f, "Pacote Debian (.deb)"),
            InstallMethod::SystemRpm => write!(f, "Pacote Red Hat / Fedora (.rpm)"),
            InstallMethod::StandaloneBinary(p) => write!(f, "Binário Local ({})", p.display()),
            InstallMethod::WindowsInstaller => write!(f, "Instalador Windows (.exe)"),
        }
    }
}

/// Normalizes and parses semantic version string into a vector of numbers
pub fn parse_version(ver: &str) -> Vec<u32> {
    let clean = ver.trim().trim_start_matches(|c| c == 'v' || c == 'V');
    clean
        .split(|c: char| c == '.' || c == '-' || c == '_')
        .filter_map(|segment| {
            let digits: String = segment.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                None
            } else {
                digits.parse::<u32>().ok()
            }
        })
        .collect()
}

/// Returns true if candidate is strictly greater than current
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    let cur_parts = parse_version(current);
    let cand_parts = parse_version(candidate);

    let max_len = cur_parts.len().max(cand_parts.len());
    for i in 0..max_len {
        let cur_val = cur_parts.get(i).copied().unwrap_or(0);
        let cand_val = cand_parts.get(i).copied().unwrap_or(0);
        if cand_val > cur_val {
            return true;
        } else if cand_val < cur_val {
            return false;
        }
    }
    false
}

/// Detects how BenderShot is installed on the current host system
pub fn detect_install_method() -> InstallMethod {
    #[cfg(target_os = "windows")]
    {
        return InstallMethod::WindowsInstaller;
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(appimage) = std::env::var("APPIMAGE") {
            let path = PathBuf::from(appimage.trim());
            if path.exists() {
                return InstallMethod::AppImage(path);
            }
        }

        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/bin/bendershot"));
        let exe_str = exe_path.to_string_lossy();

        if exe_str == "/usr/bin/bendershot" || exe_str == "/bin/bendershot" {
            // Check if managed by rpm
            if let Ok(out) = Command::new("rpm").args(["-q", "bendershot"]).output() {
                if out.status.success() {
                    return InstallMethod::SystemRpm;
                }
            }
            // Check if managed by dpkg
            if let Ok(out) = Command::new("dpkg").args(["-s", "bendershot"]).output() {
                if out.status.success() {
                    return InstallMethod::SystemDeb;
                }
            }
            return InstallMethod::StandaloneBinary(exe_path);
        }

        if let Some(base_dirs) = directories::BaseDirs::new() {
            let user_bin = base_dirs.home_dir().join(".local/bin/bendershot");
            if user_bin.exists() && exe_path == user_bin {
                return InstallMethod::StandaloneBinary(user_bin);
            }
        }

        InstallMethod::StandaloneBinary(exe_path)
    }
}

/// Selects the best matched release asset for the target install method
pub fn select_asset<'a>(assets: &'a [ReleaseAsset], method: &InstallMethod) -> Option<&'a ReleaseAsset> {
    match method {
        InstallMethod::AppImage(_) => assets.iter().find(|a| a.name.ends_with(".AppImage")),
        InstallMethod::SystemDeb => assets
            .iter()
            .find(|a| a.name.ends_with(".deb"))
            .or_else(|| assets.iter().find(|a| a.name.contains("linux-x86_64.tar.gz"))),
        InstallMethod::SystemRpm => assets
            .iter()
            .find(|a| a.name.ends_with(".rpm"))
            .or_else(|| assets.iter().find(|a| a.name.contains("linux-x86_64.tar.gz"))),
        InstallMethod::StandaloneBinary(_) => assets
            .iter()
            .find(|a| a.name.contains("linux-x86_64.tar.gz"))
            .or_else(|| assets.iter().find(|a| a.name.ends_with(".tar.gz"))),
        InstallMethod::WindowsInstaller => assets
            .iter()
            .find(|a| a.name.ends_with(".exe"))
            .or_else(|| assets.iter().find(|a| a.name.contains("Setup"))),
    }
}

/// Checks GitHub API for the latest release
pub fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    #[cfg(target_os = "windows")]
    {
        // Try curl.exe first, fallback to powershell
        if let Ok(output) = Command::new("curl.exe")
            .args([
                "-sSL",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "User-Agent: BenderShot",
                "--connect-timeout",
                "10",
                "--max-time",
                "20",
                LATEST_RELEASE_API,
            ])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Ok(info) = serde_json::from_str::<ReleaseInfo>(&text) {
                    return Ok(info);
                }
            }
        }

        // PowerShell fallback
        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; (Invoke-WebRequest -Uri '{}' -UseBasicParsing -Headers @{{'User-Agent'='BenderShot';'Accept'='application/vnd.github+json'}}).Content",
            LATEST_RELEASE_API
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output()
            .map_err(|e| format!("Falha ao invocar PowerShell para checar atualizações: {e}"))?;

        if !output.status.success() {
            return Err("Falha na requisição de atualização via PowerShell".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<ReleaseInfo>(&text)
            .map_err(|e| format!("Falha ao interpretar resposta JSON do GitHub: {e}"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("curl")
            .args([
                "-sSL",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "User-Agent: BenderShot",
                "--connect-timeout",
                "10",
                "--max-time",
                "20",
                LATEST_RELEASE_API,
            ])
            .output();

        let json_text = match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => {
                // Fallback to wget
                let wget_out = Command::new("wget")
                    .args([
                        "-qO-",
                        "--header=Accept: application/vnd.github+json",
                        "--header=User-Agent: BenderShot",
                        "--timeout=15",
                        LATEST_RELEASE_API,
                    ])
                    .output()
                    .map_err(|e| format!("Falha ao executar curl/wget para checar atualizações: {e}"))?;

                if !wget_out.status.success() {
                    return Err("Não foi possível conectar ao GitHub via curl ou wget".into());
                }
                String::from_utf8_lossy(&wget_out.stdout).to_string()
            }
        };

        serde_json::from_str::<ReleaseInfo>(&json_text)
            .map_err(|e| format!("Falha ao interpretar resposta do GitHub Releases: {e}"))
    }
}

/// Checks if there is a newer release than current version
pub fn check_for_updates() -> Result<Option<ReleaseInfo>, String> {
    let current_ver = crate::config::AppConfig::get_effective_version();
    let release = fetch_latest_release()?;

    if is_newer_version(&current_ver, &release.tag_name) {
        crate::config::AppConfig::record_update_check(Some(release.tag_name.clone()));
        Ok(Some(release))
    } else {
        crate::config::AppConfig::record_update_check(None);
        Ok(None)
    }
}

/// Downloads a remote file to a local destination path
pub fn download_file(url: &str, dest_path: &Path) -> Result<(), String> {
    if let Some(parent) = dest_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(status) = Command::new("curl.exe")
            .args(["-sSL", "-o", &dest_path.to_string_lossy(), url])
            .status()
        {
            if status.success() {
                return Ok(());
            }
        }
        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; (New-Object Net.WebClient).DownloadFile('{}', '{}')",
            url,
            dest_path.to_string_lossy()
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .status()
            .map_err(|e| format!("Falha no download via PowerShell: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("Download do arquivo de atualização falhou no PowerShell".into())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("curl")
            .args(["-sSL", "-o", dest_path.to_str().unwrap_or(""), url])
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                let wget_status = Command::new("wget")
                    .args(["-q", "-O", dest_path.to_str().unwrap_or(""), url])
                    .status()
                    .map_err(|e| format!("Falha no download via curl/wget: {e}"))?;

                if wget_status.success() {
                    Ok(())
                } else {
                    Err("Falha ao baixar arquivo de atualização".into())
                }
            }
        }
    }
}

/// Performs in-place update for current installation
pub fn perform_in_place_update<F>(release: &ReleaseInfo, progress_callback: F) -> Result<(), String>
where
    F: Fn(&str),
{
    let method = detect_install_method();
    let asset = select_asset(&release.assets, &method)
        .ok_or_else(|| format!("Nenhum pacote compatível encontrado no release {} para {}", release.tag_name, method))?;

    let temp_dir = std::env::temp_dir().join(format!("bendershot_update_{}", release.tag_name));
    let _ = std::fs::create_dir_all(&temp_dir);
    let downloaded_file = temp_dir.join(&asset.name);

    progress_callback(&format!("Baixando {}...", asset.name));
    download_file(&asset.browser_download_url, &downloaded_file)?;

    match &method {
        InstallMethod::AppImage(current_appimage_path) => {
            progress_callback("Aplicando atualização no AppImage...");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&downloaded_file, std::fs::Permissions::from_mode(0o755));
            }

            // In Linux, replacing a running binary/AppImage directly can yield ETXTBSY.
            // Moving the existing file to a backup path and moving the new one solves this atomically.
            let backup_path = current_appimage_path.with_extension("old");
            let _ = std::fs::remove_file(&backup_path);
            let _ = std::fs::rename(current_appimage_path, &backup_path);

            if let Err(e) = std::fs::copy(&downloaded_file, current_appimage_path) {
                // Restore backup
                let _ = std::fs::rename(&backup_path, current_appimage_path);
                return Err(format!("Falha ao substituir arquivo AppImage: {e}"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(current_appimage_path, std::fs::Permissions::from_mode(0o755));
            }
            let _ = std::fs::remove_file(&backup_path);
            let _ = std::fs::remove_file(&downloaded_file);
        }

        InstallMethod::StandaloneBinary(bin_path) => {
            progress_callback("Extraindo novo binário do tarball...");
            // downloaded_file is a tar.gz
            let extract_status = Command::new("tar")
                .args(["-xzf", downloaded_file.to_str().unwrap_or(""), "-C", temp_dir.to_str().unwrap_or("")])
                .status()
                .map_err(|e| format!("Falha ao extrair tarball: {e}"))?;

            if !extract_status.success() {
                return Err("Falha ao descompactar tarball de atualização".into());
            }

            // Locate new binary in temp_dir
            let mut new_bin_candidate = None;
            for entry in walk_dir_find_binary(&temp_dir, "bendershot") {
                new_bin_candidate = Some(entry);
                break;
            }

            let new_bin = new_bin_candidate.ok_or_else(|| "Binário 'bendershot' não encontrado no pacote extraído".to_string())?;

            progress_callback("Substituindo binário executável...");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&new_bin, std::fs::Permissions::from_mode(0o755));
            }

            // Check if we have write permission to the target binary directory
            let parent_writable = bin_path
                .parent()
                .map(|p| is_dir_writable(p))
                .unwrap_or(false);

            if parent_writable {
                let backup_path = bin_path.with_extension("old");
                let _ = std::fs::remove_file(&backup_path);
                let _ = std::fs::rename(bin_path, &backup_path);
                if let Err(e) = std::fs::copy(&new_bin, bin_path) {
                    let _ = std::fs::rename(&backup_path, bin_path);
                    return Err(format!("Falha ao atualizar binário: {e}"));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(bin_path, std::fs::Permissions::from_mode(0o755));
                }
                let _ = std::fs::remove_file(&backup_path);
            } else {
                // Requires pkexec / sudo
                progress_callback("Permissões de administrador necessárias. Solicitando via pkexec...");
                let status = Command::new("pkexec")
                    .args(["install", "-m", "755", new_bin.to_str().unwrap_or(""), bin_path.to_str().unwrap_or("")])
                    .status()
                    .map_err(|e| format!("Falha ao executar pkexec install: {e}"))?;

                if !status.success() {
                    return Err("Instalação do binário com privilégios de administrador cancelada ou falhou".into());
                }
            }
        }

        InstallMethod::SystemDeb => {
            progress_callback("Instalando pacote Debian (.deb) via pkexec...");
            let status = Command::new("pkexec")
                .args(["dpkg", "-i", downloaded_file.to_str().unwrap_or("")])
                .status()
                .map_err(|e| format!("Falha ao executar pkexec dpkg: {e}"))?;

            if !status.success() {
                return Err("Instalação do pacote .deb falhou ou foi cancelada pelo usuário".into());
            }
        }

        InstallMethod::SystemRpm => {
            progress_callback("Instalando pacote RPM (.rpm) via pkexec...");
            let status = Command::new("pkexec")
                .args(["rpm", "-Uvh", "--replacepkgs", downloaded_file.to_str().unwrap_or("")])
                .status()
                .map_err(|e| format!("Falha ao executar pkexec rpm: {e}"))?;

            if !status.success() {
                return Err("Instalação do pacote .rpm falhou ou foi cancelada pelo usuário".into());
            }
        }

        InstallMethod::WindowsInstaller => {
            progress_callback("Fechando o BenderShot para aplicar a atualização...");
            crate::config::AppConfig::set_installed_version(&release.tag_name);

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                const DETACHED_PROCESS: u32 = 0x00000008;

                let current_exe = std::env::current_exe()
                    .unwrap_or_else(|_| PathBuf::from("bendershot.exe"));
                let installer_str = downloaded_file.to_string_lossy();
                let exe_str = current_exe.to_string_lossy();

                // Spawn a detached script that:
                // 1. Waits briefly for this process to exit
                // 2. Force-closes any remaining bendershot.exe instances (tray daemon, editors)
                // 3. Runs the Inno Setup installer silently
                // 4. Relaunches the updated tray daemon
                let batch_cmd = format!(
                    "ping 127.0.0.1 -n 2 >nul 2>&1 & taskkill /F /IM bendershot.exe >nul 2>&1 & \"{}\" /SILENT /CLOSEAPPLICATIONS /FORCECLOSEAPPLICATIONS & start \"\" \"{}\" daemon",
                    installer_str, exe_str
                );

                Command::new("cmd.exe")
                    .args(["/C", &batch_cmd])
                    .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
                    .spawn()
                    .map_err(|e| format!("Falha ao iniciar instalador do Windows: {e}"))?;

                // Exit immediately so the running binary is unlocked for the installer
                std::process::exit(0);
            }

            #[cfg(not(target_os = "windows"))]
            {
                let status = Command::new(&downloaded_file)
                    .args(["/SILENT", "/CLOSEAPPLICATIONS", "/FORCECLOSEAPPLICATIONS"])
                    .status()
                    .map_err(|e| format!("Falha ao executar instalador do Windows: {e}"))?;

                if !status.success() {
                    return Err("Execução do instalador do Windows falhou".into());
                }
            }
        }
    }

    // Record the installed tag in configuration
    crate::config::AppConfig::set_installed_version(&release.tag_name);

    // Restart user daemon if running on Linux
    #[cfg(target_os = "linux")]
    {
        progress_callback("Reiniciando serviço daemon...");
        let _ = Command::new("systemctl")
            .args(["--user", "restart", "bendershot.service"])
            .output();
    }

    progress_callback("Atualização concluída com sucesso!");
    Ok(())
}

/// Opens a web URL in user's default browser
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd").args(["/c", "start", "", url]).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

fn walk_dir_find_binary(dir: &Path, bin_name: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().map(|n| n == bin_name).unwrap_or(false) {
                results.push(path);
            } else if path.is_dir() {
                results.extend(walk_dir_find_binary(&path, bin_name));
            }
        }
    }
    results
}

fn is_dir_writable(dir: &Path) -> bool {
    let test_file = dir.join(format!(".bendershot_write_test_{}", std::process::id()));
    if std::fs::write(&test_file, b"test").is_ok() {
        let _ = std::fs::remove_file(&test_file);
        true
    } else {
        false
    }
}
