# Windows Platform Backend

## Overview
Implements Windows screen capture (multi-monitor `xcap` + Win32 GDI `BitBlt` / `PrintWindow`), Win32 system tray icon and global hotkey daemon, JSON-backed shortcut persistence and registry autostart, and `ffmpeg` (`gdigrab`) screen recording with a native Win32 countdown HUD.

## Interface
- `[mod.rs:capture]` — Public submodule exposing Win32/DWM/xcap screen and window capture primitives.
- `[mod.rs:indicator]` — Public submodule exposing the Win32 system tray icon, global hotkey listener, and IPC message window.
- `[mod.rs:shortcuts]` — Public submodule exposing Windows shortcut persistence, hotkey parsing, registry autostart, and tray notification dispatch.
- `[mod.rs:video]` — Public submodule exposing `ffmpeg` (`gdigrab`) recording worker, countdown HUD, and stop-flag helpers.
- `[capture.rs:ensure_dpi_aware]` — Sets `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` (falling back to `SetProcessDPIAware`).
- `[capture.rs:capture]` — Async wrapper around blocking capture for `CaptureMode::Fullscreen`, `CaptureMode::Region`, and `CaptureMode::Window`.
- `[capture.rs:capture_region]` — Async multi-monitor desktop capture and region crop using `RegionSelection` and `MonitorLayout`.
- `[capture.rs:get_monitor_layouts]` — Enumerates monitors via `xcap::Monitor::all()` and computes logical/physical `MonitorLayout` entries.
- `[capture.rs:find_target_foreground_window]` — Resolves the target application `HWND` from `BENDERSHOT_TARGET_HWND`, `GetForegroundWindow`, or Z-order traversal while excluding BenderShot and system shell/overlay windows.
- `[capture.rs:load_image_from_path]` — Loads an image file into `image::RgbaImage`.
- `[indicator.rs:COPYDATASTRUCT]` — `#[repr(C)]` Win32 `WM_COPYDATA` payload structure.
- `[indicator.rs:WM_TRAYICON]` — Custom window message (`WM_APP + 100`) for tray icon mouse events.
- `[indicator.rs:WM_RELOAD_SHORTCUTS]` — Custom window message (`WM_APP + 101`) triggering hotkey re-registration.
- `[indicator.rs:hide_console_window]` — Hides and detaches the console window via `ShowWindow(SW_HIDE)` and `FreeConsole`.
- `[indicator.rs:spawn_capture]` — Spawns a `CREATE_NO_WINDOW` child process for region, full, or window capture (passing `BENDERSHOT_TARGET_HWND` for window mode).
- `[indicator.rs:show_tray_notification]` — Displays a Win32 tray balloon notification via `Shell_NotifyIconW(NIM_MODIFY)`.
- `[indicator.rs:run_indicator]` — Async entry point running a dedicated Win32 message loop for `BenderShotTrayClass`, tray icon, and global hotkeys (`RegisterHotKey`).
- `[shortcuts.rs:ShortcutManager]` — Struct providing `detect_conflict`, `get_current_binding`, `set_binding`, `get_current_window_binding`, `set_window_binding`, `get_current_region_binding`, `set_region_binding`, `get_current_record_binding`, `set_record_binding`, `parse_hotkey`, `configure_autostart`, `is_autostart_enabled`, `start_daemon_if_not_running`, `send_notification`, and Linux-parity no-op stubs (`disable_gnome_screenshot_ui`, `restore_gnome_screenshot_ui`, `is_gnome_screenshot_ui_disabled`, `is_shortcut_registered`, `is_running_as_appimage`, `integrate_appimage`).
- `[video.rs:pid_file_path]` — Returns temp path `bendershot-record.pid`.
- `[video.rs:temp_video_path]` — Returns temp path `bendershot-rec.mp4`.
- `[video.rs:video_path_file]` — Returns temp path `bendershot-rec-path.txt`.
- `[video.rs:stop_flag_file]` — Returns temp path `bendershot-stop-record.flag`.
- `[video.rs:is_process_running]` — Queries Win32 `GetExitCodeProcess` against `STILL_ACTIVE` (259).
- `[video.rs:stop_worker_process]` — Writes `stop` to `bendershot-stop-record.flag`.
- `[video.rs:send_notification]` — Delegates to `ShortcutManager::send_notification`.
- `[video.rs:detect_screen_dimensions]` — Queries `GetSystemMetrics(SM_CXSCREEN / SM_CYSCREEN)` with `(1920, 1080)` fallback.
- `[video.rs:run_recorder_worker]` — Runs a 3-second Win32 GDI `BenderShotCountdownHUD` banner, then spawns `ffmpeg -f gdigrab -i desktop` until the stop flag appears.

## Invariants
- `[capture.rs:ensure_dpi_aware]` — Must be invoked before querying monitor coordinates, virtual screen metrics, or window rects so coordinates match physical pixels.
- `[capture.rs:capture_active_window]` — Temporarily hides visible `bendershot.exe` windows (excluding `BenderShotTrayClass`) and restores them after capture so BenderShot's own UI never occludes the target window.
- `[capture.rs:capture_hwnd_print_window]` — Rejects `PrintWindow` results with fewer than 5% non-black pixels so failed hardware-accelerated client captures fall back to `BitBlt`.
- `[indicator.rs:run_indicator]` — Closes any pre-existing `BenderShotTrayClass` window via `WM_CLOSE` before registering the new tray icon and global hotkeys.
- `[video.rs:run_recorder_worker]` — Stops `ffmpeg` gracefully by writing `q\n` to its piped `stdin` and awaiting exit, removing `bendershot-stop-record.flag` afterward.

## Side Effects
- `[capture.rs:ensure_dpi_aware]` — Mutates process-wide Win32 DPI awareness context.
- `[capture.rs:capture_active_window]` — Calls `ShowWindow(SW_HIDE / SW_SHOWNOACTIVATE)` on BenderShot windows and `SetForegroundWindow` on the target window.
- `[indicator.rs:run_indicator]` — Detaches console (`FreeConsole`), caches `bendershot.ico` in `ProjectDirs::config_dir()`, registers/unregisters system tray icon and global hotkeys, and spawns child processes with `CREATE_NO_WINDOW`.
- `[shortcuts.rs:ShortcutManager]` — Reads/writes `shortcuts.json` in `ProjectDirs::config_dir()`, modifies `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` via `reg`, and sends `WM_RELOAD_SHORTCUTS` / `WM_COPYDATA` to `BenderShotTrayClass`.
- `[video.rs:run_recorder_worker]` — Creates a `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` countdown window, writes `bendershot-rec-path.txt`, manages `bendershot-stop-record.flag`, and spawns `ffmpeg` writing `bendershot-rec.mp4`.

## Verification
- Run: `cargo test`
