# XDG Portal Permission Pre-Authorization, Capture Retry, and Local Build Workflow

## Context
1. **Intermittent Two-Press PrintScreen Failure**:
   - In GNOME Wayland desktop sessions, users observed that pressing <kbd>PrintScreen</kbd> occasionally required two key presses for the capture to actually take place, even when running the resident background daemon (`bendershot daemon`).
   - Deep inspection of the user session system logs revealed the underlying root cause:
     ```text
     /usr/libexec/xdg-desktop-portal[1412149]: Failed to show access dialog: GDBus.Error:org.freedesktop.DBus.Error.AccessDenied: Only the focused app is allowed to show a system access dialog
     bendershot[1865538]: Error: Empty response from screenshot portal
     bendershot[1865538]: Caused by: Portal request didn't succeed: Other
     ```
   - When `bendershot region` was launched in the background by `gsd-media-keys`, it lacked explicit permission in `org.freedesktop.impl.portal.PermissionStore` (table `screenshot`). As a result, `xdg-desktop-portal` attempted to display an access confirmation prompt to the user.
   - However, modern GNOME Shell / Mutter blocks background/unfocused processes from spawning access dialogs to prevent focus stealing and phish-prompting. Because the launched process had no focused window surface, Mutter rejected the dialog, aborting the capture on the first press.
   - On the second key press (or subsequent runs), either a temporary token/activity timestamp or previous grant allowed the call to succeed.
   - Furthermore, `capture.rs` had no request timeout (risking indefinitely blocked processes on dropped portal signals) and no automatic retry mechanism.

2. **Daemon Concurrency & Hanging Process Cleanups**:
   - Multiple daemon instances could be started concurrently (e.g. from autostart, systemd user unit, or CLI) without single-instance mutual exclusion.
   - A stuck capture process could linger in `futex_do_wait` if a portal signal was dropped, holding session resources.

3. **Local Build & Packaging Script**:
   - Users requested a unified root script (`./build_local.sh`) to reliably compile release/debug binaries, run test suites, and generate distribution packages (`.rpm`, `.deb`, `.AppImage`, `.tar.gz`) into `packaging/dist`.

## Decisions

1. **Pre-Authorization in XDG Desktop Portal PermissionStore**:
   - Proactively register authorization (`"yes"`) for `bendershot` (as well as legacy alias `enhanced-shot` and unconfined fallbacks) in `org.freedesktop.impl.portal.PermissionStore` table `screenshot`, id `screenshot`.
   - Invoke `ensure_portal_permissions()`:
     - During `bendershot auto-setup` and `ShortcutManager::set_binding()`.
     - At the start of `bendershot::platform::linux::capture::capture()`.
     - In post-installation scripts for RPM (`bendershot.spec`), Debian (`packaging/package.sh`), and the universal installer (`install.sh`).
   - Eliminating the interactive access dialog allows instant background captures on the very first key press.

2. **Capture Timeout and Automatic Retry Loop**:
   - Implement a 5-second `tokio::time::timeout` on every portal screenshot request to prevent indefinite process hanging.
   - Wrap portal requests in a resilient retry loop (up to 3 attempts with 150ms backoff).
   - If an initial attempt fails due to D-Bus latency or a portal synchronization glitch, `capture()` automatically refreshes portal permissions, retries, and returns the screenshot seamlessly on the same user key press.

3. **Single-Instance Enforcement & Stale Process Lifecycle**:
   - Implement PID-file tracking in `XDG_RUNTIME_DIR/bendershot-capture.pid`. Before a new capture begins, any stale/hanging prior capture process is terminated via `SIGTERM` / `SIGKILL`.
   - Implement daemon single-instance check in `XDG_RUNTIME_DIR/bendershot-daemon.pid`. When `bendershot daemon` is started while another daemon instance is already active, the new instance logs an informational message and exits cleanly (exit code 0).

4. **Root Build & Packaging Script (`./build_local.sh`)**:
   - Create an executable, self-contained bash script at the project root with the following options:
     - `./build_local.sh`: Builds release binary with `cargo build --release` and runs packaging for `.rpm`, `.deb`, `.AppImage`, and `.tar.gz`.
     - `-q, --quick`: Compiles release binary only (skipping packaging).
     - `-d, --debug`: Compiles debug binary.
     - `-t, --test`: Runs the unit and integration test suite before building.
     - `-i, --install`: Installs the compiled binary locally to `~/.local/bin/bendershot`, installs icons, and reloads systemd user daemon.
