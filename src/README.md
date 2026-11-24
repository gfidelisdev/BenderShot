# BenderShot Core (`src`)

## Overview
Core Rust crate (`bendershot`) implementing the CLI entrypoint, configuration persistence, cross-platform screen/region capture orchestration, vector annotation scene model, rasterization and obfuscation engine, local single-instance IPC, GitHub in-place updater, and FFmpeg video recording/export pipeline.

## Interface
- `[lib.rs:pub mod]`: Re-exports `capture`, `cli`, `config`, `export`, `indicator`, `ipc`, `model`, `platform`, `rasterize`, `shortcuts`, `ui`, `smooth`, `updater`, and `video`.
- `[main.rs:main]`: Tokio async binary entrypoint invoking `real_main()` and reporting fatal errors via `report_fatal_error()`.
- `[cli.rs:Cli]` / `[cli.rs:Commands]`: `clap` parser and subcommands (`Region`, `Full`, `Window`, `Daemon`, `Edit`, `Record`, `Settings`, `Wizard`, `AutoSetup`, `Update`, hidden `RecordWorker`).
- `[config.rs:AppConfig]` / `[config.rs:ToolbarPosition]`: Serializable `config.json` state, shortcut defaults, update metadata, and toolbar layout preferences.
- `[capture.rs:CaptureBackend]` / `[capture.rs:CaptureMode]` / `[capture.rs:RegionSelection]` / `[capture.rs:MonitorLayout]`: Screen/region capture facade and pure helpers `crop_selection_from_desktop` and `trim_transparent`.
- `[model.rs:Scene]` / `[model.rs:AnnotationItem]` / `[model.rs:AnnotationSpan]` / `[model.rs:TimelineItem]` / `[model.rs:ToolMode]`: Non-destructive vector annotation scene graph and video timeline spans.
- `[rasterize.rs:rasterize_scene]` / `[rasterize.rs:rasterize_scene_at_time]` / `[rasterize.rs:apply_obfuscation]` / `[rasterize.rs:apply_smudge_step]` / `[rasterize.rs:apply_color_erase]`: Two-pass software rasterizer and pixel manipulation primitives.
- `[smooth.rs:smooth_stroke]` / `[smooth.rs:distance_filter]` / `[smooth.rs:apply_chaikin]`: Freehand stroke smoothing pipeline.
- `[export.rs:Exporter]`: Clipboard copy and PNG/file export utilities.
- `[ipc.rs:IpcServerGuard]` / `[ipc.rs:is_editor_active]` / `[ipc.rs:try_send_capture]` / `[ipc.rs:try_send_file_path]` / `[ipc.rs:start_editor_ipc_listener]`: Local single-instance editor IPC over Unix domain socket or Windows localhost TCP port file.
- `[indicator.rs:run_indicator]` / `[shortcuts.rs:ShortcutManager]` / `[shortcuts.rs:KeybindingConflict]`: Cross-platform facades delegating to `crate::platform::imp`.
- `[updater.rs:ReleaseInfo]` / `[updater.rs:InstallMethod]` / `[updater.rs:check_for_updates]` / `[updater.rs:perform_in_place_update]`: GitHub release discovery and in-place self-update engine.
- `[video.rs:ScreenRecorder]` / `[video.rs:VideoExporter]`: Background recording lifecycle and FFmpeg/FFprobe frame-level annotated video export.
- [Platform Backends](./platform/README.md) — Compile-time OS abstraction (`linux` and `windows` backends).
- [User Interfaces](./ui/README.md) — `eframe`/`egui` editor, region selector, pinned window, settings, wizard, updater UI, and Libadwaita theme.

## Invariants
- `[main.rs:real_main]`: Defaults to `Commands::Wizard` when `!AppConfig::is_setup_completed()`, otherwise `Commands::Region`; sleeps 80ms after `ui::run_region_selector()` before capturing.
- `[config.rs:AppConfig::load]`: Falls back to `AppConfig::default()` if `config.json` cannot be read or deserialized.
- `[capture.rs:crop_selection_from_desktop]`: Matches `overlay_size` against `MonitorLayout` within 4.0 logical points (preferring primary monitor on ambiguity) and returns `None` if cropped dimensions are `<= 1` pixel.
- `[model.rs:Scene]`: Keeps `items` and `spans` parallel-indexed and maintains `next_badge_number` as `max(StepBadge.number) + 1` (minimum `1`).
- `[rasterize.rs:rasterize_scene_at_time]`: `time_ms == u64::MAX` disables time filtering (`progress = 1.0`, `alpha = 1.0`); `Obfuscate` items always render in pass 1 before vector annotations in pass 2.
- `[ipc.rs:start_editor_ipc_listener]`: Refuses to bind (returns `None`) if `is_editor_active()` returns `true`.
- `[updater.rs:perform_in_place_update]`: Requires a matching asset from `select_asset` and backs up writable binaries to `.old` before replacing.
- `[video.rs:ScreenRecorder::stop_recording]`: Polls worker PID up to 150 times at 100ms intervals (15s max) before cleaning up `pid_file_path()`.

## Side Effects
- `[config.rs:AppConfig::save]`: Creates parent directories and writes pretty-printed JSON to `config.json`.
- `[export.rs:Exporter]`: Writes RGBA images to system clipboard (`arboard`) and PNG files on disk.
- `[ipc.rs]`: Creates/removes Unix socket (`$XDG_RUNTIME_DIR/bendershot-editor.sock`) or Windows port file (`bendershot-editor.port`) and temporary PNGs (`bendershot-ipc-*.png`).
- `[rasterize.rs:load_first_available]`: Reads system TrueType font files from disk into `OnceLock` caches and mutates `RgbaImage` buffers in place.
- `[updater.rs]`: Queries GitHub Releases API via `curl`/`wget`/PowerShell, writes temporary update artifacts, invokes `pkexec`/`systemctl`/`cmd.exe` for privileged or Windows updates, and updates `AppConfig`.
- `[video.rs]`: Spawns `record-worker`, `ffmpeg`, and `ffprobe` subprocesses, manages PID/video-path files, and creates/deletes temporary frame-cache directories.

## Verification
- Run: `cargo test`
