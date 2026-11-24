# Platform Abstraction Layer

## Overview
Selects and re-exports the OS-specific backend (`linux` or `windows`) as `crate::platform::imp` via compile-time `#[cfg(target_os = ...)]` attributes, providing a uniform interface for capture, tray indicator, shortcuts, and video recording across supported platforms.

## Interface
- `[mod.rs:linux]` — Public module compiled when `target_os = "linux"`; see [./linux/README.md](./linux/README.md).
- `[mod.rs:windows]` — Public module compiled when `target_os = "windows"`; see [./windows/README.md](./windows/README.md).
- `[mod.rs:imp]` — Public alias re-exporting either `linux` or `windows` as the active platform implementation.

## Invariants
- `[mod.rs:imp]` — Both `linux` and `windows` modules must expose matching submodules (`capture`, `indicator`, `shortcuts`, `video`) and compatible public entry points consumed by `crate::capture`, `crate::shortcuts`, `crate::video`, and `main`.

## Side Effects
- None directly in `[mod.rs]`; all runtime side effects are delegated to the active `imp` backend ([./linux/README.md](./linux/README.md) or [./windows/README.md](./windows/README.md)).

## Verification
- Run: `cargo test`
