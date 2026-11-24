# Linux Platform Backend

## Overview
Implements Linux (Wayland/GNOME) screen capture, live region selection, system tray indicator daemon, GNOME keybinding and autostart management, and screen recording. Relies on XDG Desktop Portal (`ashpd`), `gsettings`, `systemctl --user`, `ksni` StatusNotifierItem, and an embedded Python 3 GTK3/GStreamer/Mutter D-Bus helper (`gnome_helper.py`).

## Interface
- `[mod.rs:capture]` — Public submodule exposing Wayland/GNOME screenshot and region selection primitives.
- `[mod.rs:indicator]` — Public submodule exposing the `ksni` tray icon and background daemon loop.
- `[mod.rs:shortcuts]` — Public submodule exposing GNOME `gsettings` keybinding, autostart, AppImage integration, and notification management.
- `[mod.rs:video]` — Public submodule exposing GNOME Shell Screencast video recording worker and PID helpers.
- `[capture.rs:ensure_portal_permissions]` — Pre-registers `"yes"` in `org.freedesktop.impl.portal.PermissionStore` for `bendershot`, `enhanced-shot`, `io.github.whattytheshell`, and `""` via `busctl`.
- `[capture.rs:cleanup_stale_capture_processes]` — Terminates stale capture PIDs recorded in `bendershot-capture.pid` unless an editor is active, then writes current PID.
- `[capture.rs:remove_capture_pid_file]` — Removes `bendershot-capture.pid` from `XDG_RUNTIME_DIR` (or temp dir).
- `[capture.rs:is_process_alive]` — Checks process liveness via `kill -0 <pid>`.
- `[capture.rs:capture]` — Async entry point for `CaptureMode::Fullscreen`, `CaptureMode::Region`, and `CaptureMode::Window` (tries Mutter ScreenCast focused window first, falls back to portal desktop).
- `[capture.rs:select_region_live]` — Spawns `python3 -W ignore -c <gnome_helper.py> select` and parses JSON `Option<RegionSelection>`.
- `[capture.rs:capture_region]` — Captures full desktop via XDG portal, queries monitor layout, and crops `RegionSelection`.
- `[capture.rs:load_image_from_path]` — Loads an image file into `image::RgbaImage`.
- `[gnome_helper.py:window]` — CLI subcommand (`window OUT`) capturing the focused window via `org.gnome.Mutter.ScreenCast` and GStreamer `pipewiresrc` into PNG.
- `[gnome_helper.py:monitors]` — CLI subcommand (`monitors`) printing logical monitor layout from `org.gnome.Mutter.DisplayConfig` as JSON.
- `[gnome_helper.py:select]` — CLI subcommand (`select [x0,y0,x1,y1]`) displaying a GTK3/Cairo transparent X11 (`GDK_BACKEND=x11`) overlay across monitors and printing selected `rect` and `overlay_size` JSON.
- `[indicator.rs:BenderShotTray]` — Struct implementing `ksni::Tray` with capture, record, wizard, settings, update, and exit menu items.
- `[indicator.rs:run_indicator]` — Async daemon entry point enforcing single-instance PID lock, auto-configuring shortcuts on first run, spawning 12h update checks, and serving `BenderShotTray` (with 15s retry loop if `StatusNotifierWatcher` is absent).
- `[shortcuts.rs:ShortcutManager]` — Struct providing `detect_conflict`, `get_current_binding`, `set_binding`, `get_current_window_binding`, `set_window_binding`, `get_current_region_binding`, `set_region_binding`, `get_current_record_binding`, `set_record_binding`, `disable_gnome_screenshot_ui`, `restore_gnome_screenshot_ui`, `is_gnome_screenshot_ui_disabled`, `is_shortcut_registered`, `configure_autostart`, `is_autostart_enabled`, `start_daemon_if_not_running`, `is_running_as_appimage`, `integrate_appimage`, and `send_notification`.
- `[shortcuts.rs:get_executable_command]` — Resolves the preferred BenderShot executable path (`APPIMAGE`, `current_exe`, `~/.local/bin/bendershot`, `/usr/bin/bendershot`, or `"bendershot"`).
- `[shortcuts.rs:reload_media_keys_daemon]` — Restarts `org.gnome.SettingsDaemon.MediaKeys.target` via `systemctl --user`.
- `[shortcuts.rs:unbind_conflicting_system_shortcuts]` — Clears conflicting keybindings across GNOME Shell, WM, and media-keys schemas.
- `[video.rs:pid_file_path]` — Returns temp path `bendershot-record.pid`.
- `[video.rs:temp_video_path]` — Returns temp path `bendershot-rec.mp4`.
- `[video.rs:video_path_file]` — Returns temp path `bendershot-rec-path.txt`.
- `[video.rs:is_process_running]` — Checks `/proc/<pid>` existence.
- `[video.rs:stop_worker_process]` — Sends `kill -TERM <pid>`.
- `[video.rs:send_notification]` — Invokes `notify-send` with BenderShot app name and icon.
- `[video.rs:detect_screen_dimensions]` — Parses `xdpyinfo` output or defaults to `(1920, 1080)`.
- `[video.rs:run_recorder_worker]` — Displays a 3-second `notify-send` countdown, spawns Python `org.gnome.Shell.Screencast` D-Bus recorder, and forwards `SIGTERM`/`SIGINT` to stop cleanly.

## Invariants
- `[capture.rs:cleanup_stale_capture_processes]` — Must never terminate a stale capture PID when `crate::ipc::is_editor_active()` returns `true`.
- `[capture.rs:portal_capture_desktop]` — Non-interactive XDG portal requests are capped at 3 attempts with a 5-second timeout per attempt so background invocations never hang indefinitely.
- `[gnome_helper.py:select_region]` — Forces `GDK_BACKEND=x11` for GTK3 `Gtk.WindowType.POPUP` so global coordinates, pointer/keyboard seat grabs, and multi-monitor spans behave consistently under XWayland.
- `[indicator.rs:run_indicator]` — Enforces single-instance execution via `bendershot-daemon.pid` in `XDG_RUNTIME_DIR` before spawning background tasks or tray registration.
- `[shortcuts.rs:ShortcutManager]` — Setting any BenderShot shortcut (`set_binding`, `set_window_binding`, `set_region_binding`, `set_record_binding`) removes legacy `enhanced-shot` slots, unbinds conflicting custom/system shortcuts, and restarts `org.gnome.SettingsDaemon.MediaKeys.target`.

## Side Effects
- `[capture.rs:ensure_portal_permissions]` — Mutates `org.freedesktop.impl.portal.PermissionStore` via `busctl --user`.
- `[capture.rs:cleanup_stale_capture_processes]` — Reads/writes `bendershot-capture.pid` and sends `SIGTERM`/`SIGKILL` to stale processes.
- `[capture.rs:capture_focused_window]` — Creates and deletes temporary PNG `bendershot-window-<pid>.png` in `std::env::temp_dir()`.
- `[indicator.rs:run_indicator]` — Writes/removes `bendershot-daemon.pid`, mutates GNOME shortcuts and autostart on first run, spawns periodic update check tasks, and spawns child `bendershot` processes on tray menu actions.
- `[shortcuts.rs:ShortcutManager]` — Executes `gsettings`, `systemctl --user`, `notify-send`, `update-desktop-database`, and `gtk-update-icon-cache`; writes `.desktop` files, icons, and symlinks (`~/.local/bin/bendershot`) during autostart/AppImage integration.
- `[video.rs:run_recorder_worker]` — Invokes `notify-send` and `gdbus` notifications, spawns `python3` calling `org.gnome.Shell.Screencast`, and writes the recorded video path to `bendershot-rec-path.txt`.

## Verification
- Run: `cargo test`
