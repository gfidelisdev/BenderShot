# UI (`src/ui`)

## Overview
The `src/ui` component implements BenderShot's graphical user interfaces using `eframe` / `egui` styled with a GNOME Libadwaita-inspired theme. It provides the main multi-tab screenshot and video annotation editor, a live-screen region selection overlay, an always-on-top pinned screenshot viewer, a global shortcut and update settings window, a first-run setup wizard, and an in-place updater UI.

## Interface
- `[mod.rs:load_app_icon]`: Decodes the embedded PNG icon (`../../packaging/assets/bendershot.png`) into `Option<egui::IconData>`.
- `[mod.rs:run_editor]`, `[mod.rs:run_pin_window]`, `[mod.rs:run_region_selector]`, `[mod.rs:run_settings_window]`, `[mod.rs:run_updater_window]`, `[mod.rs:run_wizard_window]`, `[mod.rs:apply_adwaita_theme]`, `[mod.rs:AdwaitaThemeMode]`: Re-exports of primary window entrypoints and theme utilities.
- `[editor.rs:EditorApp]`, `[editor.rs:EditorTab]`, `[editor.rs:ShortcutTarget]`: Multi-tab image/video annotation editor state, canvas rendering (zoom, pan, loupe, vector annotations, raster obfuscation/smudge/color-erase, crop, layers panel, video timeline, inline text, and settings modal), and `eframe::App` implementation.
- `[editor.rs:EditorApp::new]`, `[editor.rs:EditorApp::with_ipc]`: Constructors for standalone or IPC-connected editor instances.
- `[editor.rs:EditorApp::switch_tab]`, `[editor.rs:EditorApp::close_tab]`, `[editor.rs:EditorApp::add_new_tab]`, `[editor.rs:EditorApp::add_new_tab_with_title]`, `[editor.rs:EditorApp::save_active_tab_state]`, `[editor.rs:EditorApp::load_active_tab_state]`: Tab lifecycle and state-swapping methods.
- `[editor.rs:EditorApp::fit_to_viewport]`, `[editor.rs:EditorApp::set_zoom_100]`, `[editor.rs:EditorApp::zoom_by]`, `[editor.rs:EditorApp::clamp_pan]`, `[editor.rs:EditorApp::finish_current_shape]`, `[editor.rs:EditorApp::set_toolbar_position]`, `[editor.rs:EditorApp::toggle_toolbar_expanded]`: Viewport navigation, shape finalization, and toolbar layout controls.
- `[editor.rs:run_editor]`, `[editor.rs:run_video_editor]`: Native `eframe` window launchers for image editing and video annotation/playback.
- `[pin.rs:PinApp]`, `[pin.rs:run_pin_window]`: Borderless, transparent, always-on-top draggable window displaying a pinned `RgbaImage` (closed via `Escape`).
- `[region_selector.rs:RegionSelectorApp]`, `[region_selector.rs:run_region_selector]`: Live-screen region selection overlay returning `Option<RegionSelection>` (on Linux, delegates to `crate::platform::linux::capture::select_region_live` first and falls back to the `eframe` overlay on error).
- `[settings.rs:SettingsApp]`, `[settings.rs:run_settings_window]`: Standalone settings window for recording/editing global shortcuts, detecting shortcut conflicts, toggling auto-update checks, and launching the setup wizard or updater GUI.
- `[theme.rs:AdwaitaThemeMode]`, `[theme.rs:apply_adwaita_theme]`, `[theme.rs:setup_fonts]`: Libadwaita dark/light `egui::Visuals` configuration and one-time system font/emoji fallback registration.
- `[updater_ui.rs:UpdaterApp]`, `[updater_ui.rs:UpdaterEvent]`, `[updater_ui.rs:run_updater_window]`: Asynchronous GitHub release checker and in-place updater UI.
- `[wizard.rs:SetupWizardApp]`, `[wizard.rs:run_wizard_window]`: First-run setup wizard for configuring global shortcuts, autostart/daemon mode, and Linux AppImage integration.

## Invariants
- `[theme.rs:setup_fonts]`: System font discovery and `ctx.set_fonts` registration run at most once per process via `FONT_INIT` (`std::sync::Once`).
- `[editor.rs:EditorApp::switch_tab]`: Switching or adding tabs must call `commit_inline_text`, clear transient drag/freehand state, and swap active editor fields via `save_active_tab_state` / `load_active_tab_state`.
- `[editor.rs:EditorApp::close_tab]`: Closing the last remaining tab sends `egui::ViewportCommand::Close` instead of leaving `tabs` empty.
- `[editor.rs:EditorApp::save_snapshot_for_undo]`: Destructive raster operations (`Blur`, `Pixelate`, `Smudge`, `ColorErase`, `Crop`) and annotation additions/removals/reordering push an `EditorStateSnapshot` to `undo_stack` and clear `redo_stack`.
- `[region_selector.rs:RegionSelectorApp::close]`: Sets `self.closing = true` and sends `ViewportCommand::Close` directly without `ViewportCommand::Visible(false)` to avoid Win32 `WM_PAINT` starvation.
- `[region_selector.rs:RegionSelectorApp::confirm]`: Only records a `RegionSelection` if width and height are both at least `MIN_SIZE` (`8.0`).
- `[settings.rs:SettingsApp::update]`, `[editor.rs:EditorApp::update]`: Global editor shortcuts are suppressed while `recording_shortcut_target` is `Some(_)`.

## Side Effects
- `[editor.rs:run_editor]`, `[editor.rs:run_video_editor]`: Starts a local IPC listener (`crate::ipc::start_editor_ipc_listener`) and, in video mode, creates a temporary frame-cache directory under `std::env::temp_dir()` and spawns `ffmpeg` (`ScreenRecorder::ffmpeg_bin()`) to extract playback frames at `PLAYBACK_FPS` (`10`).
- `[editor.rs:EditorApp]`: Reads/writes `AppConfig` (`toolbar_position`, `toolbar_expanded`, `auto_check_updates`, `latest_known_tag`), reads/writes the system clipboard and auto-saves images via `Exporter`, spawns `zenity` for file open dialogs, invokes `VideoExporter::export_annotated_video`, and spawns `current_exe()` with `["update", "--gui"]`.
- `[editor.rs:EditorApp::update]`: Drains IPC payloads, deletes temporary capture files when `delete_file_after_load` is set, and optionally spawns a background thread to check GitHub updates when the 4-hour interval has elapsed.
- `[theme.rs:setup_fonts]`: Reads system font files from `/usr/share/fonts/...` or `C:\Windows\Fonts\...` on disk.
- `[settings.rs:SettingsApp]`, `[wizard.rs:SetupWizardApp]`: Mutates desktop environment shortcut bindings, autostart configuration, daemon process state, and AppImage integration via `ShortcutManager`, persists `AppConfig`, sends desktop notifications, and spawns `current_exe()` for `wizard` or `update --gui`.
- `[updater_ui.rs:UpdaterApp]`: Spawns background threads for network update checks (`check_for_updates`), in-place binary replacement (`perform_in_place_update`), and opening release URLs (`crate::updater::open_url`).
- `[region_selector.rs:run_region_selector]`: Writes the confirmed `RegionSelection` into a shared `Arc<Mutex<Option<RegionSelection>>>`.

## Verification
- Run: `cargo test`
