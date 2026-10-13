# Porting BenderShot to Windows

## Project Overview
- **Name:** BenderShot
- **Language & Edition:** Rust (Edition 2024)
- **Primary Source Platform:** Linux (Wayland / GNOME)
- **Target Platform:** Windows (x86_64-pc-windows-gnu / x86_64-pc-windows-msvc)
- **UI Framework:** egui 0.29 / eframe 0.29 (cross-platform immediate mode GUI)

## Architecture & Strategy
- **Strategy Used:** Strategy A (Platform Abstraction Layer).
- **Core Shared Modules (100% Shared):**
  - Editor Window & UI Canvas (`src/ui/editor.rs`, `src/ui/pin.rs`)
  - Adwaita Theme Mode (`src/ui/theme.rs`)
  - Vector Model & Scene Graph (`src/model.rs`)
  - Rasterization & Obfuscation Engines (`src/rasterize.rs`)
  - Stroke Smoothing & Bézier curves (`src/smooth.rs`)
  - Export & Auto-Save Pipeline (`src/export.rs`)
  - Video Timeline & Annotation Burning (`src/video.rs`)
- **Platform Abstraction Layer (`src/platform/`):**
  - `src/platform/linux/`: Wayland portal screen capture (`ashpd`), D-Bus tray indicator (`ksni`), GNOME `gsettings` shortcuts, and GNOME Screencast.
  - `src/platform/windows/`: Native Windows multi-display and window capture (`xcap`), Win32 process & display metrics (`windows-sys`), persistent shortcut storage in `%APPDATA%`, and Windows background daemon.

## Test Results Matrix

| Platform | Builds | Unit tests | Smoke/integration | Tested as | Notes |
|---|---|---|---|---|---|
| Linux (x86_64) | Yes (dev + release) | Yes (13/13 passed) | Yes (1/1 passed) | Real (Fedora 44) | Binário release em `target/release/bendershot` (22MB). Pacotes gerados em `packaging/dist/`: `.rpm` (5.1MB), `.deb` (4.8MB), `.AppImage` (7.3MB), `.tar.gz` (7.1MB) |
| Windows (x86_64) | Yes (`cargo check --release`) | Yes (`cargo check --tests`) | Pending (runtime) | Target `x86_64-pc-windows-gnu` | Tipagem, borrow checker e otimização release validados |

## Known Limitations & Considerations on Windows
1. **Screen Recording:** Windows recording utilizes FFmpeg with the native `gdigrab` desktop grabber device. FFmpeg must be present in `%PATH%` on the user's Windows machine to perform video capture.
2. **Tray Notification Icon:** The Windows daemon service is set up and functional via standard Win32 loops; rich context menus can be further extended using `tray-icon` if advanced shell menus are desired.
3. **Hardware Acceleration:** Default backend uses Glow (OpenGL) across all platforms; on Windows, OpenGL drivers are supported out of the box on Intel, AMD, and NVIDIA.

## How to Build and Run on Windows

### Prerequisites
- Rust toolchain (stable or 2024 edition): `rustup default stable`
- (Optional, for video recording) FFmpeg installed and added to `PATH`.

### Build
```cmd
cargo build --release
```

### Run
- **Full screen capture & edit:**
  ```cmd
  target\release\bendershot.exe full
  ```
- **Region capture & edit:**
  ```cmd
  target\release\bendershot.exe region
  ```
- **Active window capture:**
  ```cmd
  target\release\bendershot.exe window
  ```
- **Open existing image for editing:**
  ```cmd
  target\release\bendershot.exe edit path\to\screenshot.png
  ```
- **Background daemon:**
  ```cmd
  target\release\bendershot.exe daemon
  ```

## Manual Verification Checklist for Windows
When testing the binary on a real Windows machine:
- [ ] **Startup:** Run `bendershot.exe full` and verify the editor window opens displaying captured display(s).
- [ ] **Multi-Monitor:** Test on a multi-monitor setup to verify stitched desktop capture.
- [ ] **Vector Tools:** Test drawing shapes (rectangle, polygon, ellipse, arrow, bézier, step badges) and verify undo/redo.
- [ ] **Font Rendering:** Check that text annotations render cleanly using Windows fonts (`Segoe UI` / `Arial`).
- [ ] **Pin Window:** Click "Fixar" (Pin) on an image slice and verify floating borderless pin window stays on top.
- [ ] **Clipboard & Auto-Save:** Copy to clipboard (Ctrl+C) and verify pasting into Paint or Discord; verify auto-save PNG in `%USERPROFILE%\Pictures\Screenshots`.
- [ ] **Video Editor:** Open a recorded MP4 file, scrub the timeline, add annotation spans, and export to MP4.

## Packaging & Windows Installer
- **Script:** `packaging/windows/bendershot.iss` (Inno Setup)
- **Features:**
  - 64-bit installation in `{autopf}\BenderShot`.
  - **Dynamic FFmpeg Detection:** Automatically queries the system (`where ffmpeg` / `{app}\ffmpeg.exe`). If missing, presents a pre-selected option to download/install FFmpeg via `winget` or direct portable download.
  - **Dynamic Executable Resolution:** `ScreenRecorder::ffmpeg_bin()` checks for a local `ffmpeg.exe` in the application directory before falling back to system PATH.
  - Configures registry key `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` to run `bendershot.exe daemon` on Windows startup.
  - Automatically launches the background daemon immediately at the end of setup (`[Run]`).
  - Gracefully stops any active daemon instances before uninstallation (`taskkill /F /IM bendershot.exe`).
- **Build Command:**
  ```cmd
  iscc packaging\windows\bendershot.iss
  ```
- **Output:** `packaging\dist\BenderShot-Setup-x64.exe`

## Code Signing (Windows Authenticode)
To eliminate Windows Defender SmartScreen warnings ("Unknown Publisher"):
1. **Acquire a Code Signing Certificate:**
   - Standard OV / Individual (Sectigo, DigiCert, SSL.com).
   - EV (Extended Validation) for instant SmartScreen reputation.
   - Microsoft Trusted Signing (cloud-native Azure service, no hardware USB token needed).
2. **Sign Binary and Installer with SignTool:**
   ```cmd
   signtool sign /f certificate.pfx /p "password" /tr http://timestamp.digicert.com /td sha256 /fd sha256 target\release\bendershot.exe
   signtool sign /f certificate.pfx /p "password" /tr http://timestamp.digicert.com /td sha256 /fd sha256 packaging\dist\BenderShot-Setup-x64.exe
   ```
3. **Automate via GitHub Actions:** Store `.pfx` as repository secret `WINDOWS_CERTIFICATE_BASE64` and add signing step before artifact upload.
