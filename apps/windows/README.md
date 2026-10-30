# Zoomouse Windows 11 — M1-C user-session tray/settings scaffold

`apps/windows` contains an independent Tauri 2 + React/TypeScript desktop project with a system-tray icon generated in memory, local session status and a one-way emergency-stop/revoke action.

The native app owns a `Mutex<HostSession<DisabledInput>>`. Its Tauri commands are deliberately limited to `host_status` and `emergency_stop`. There is **no** approve, remote network, shell, filesystem or process-execution command, no elevation, Session 0 service, Win32 input injection or screen capture. The UI explicitly labels LAN pairing, remote input and capture as **not implemented**. The emergency-stop action denies further input and invokes backend cleanup.

## Development prerequisites / commands

Windows 11 interactive user account, Rust MSVC toolchain, C++ build tools, WebView2, pnpm 10 and Tauri 2 prerequisites. After parent integrates this workspace's dependency/lockfile updates: `pnpm --filter @zoomouse/windows typecheck`, `pnpm --filter @zoomouse/windows build`, and from `apps/windows` `pnpm tauri dev`. Rust host pure tests: `cargo test --manifest-path crates/host/Cargo.toml`. Tauri Rust compile: `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml`.

The root `pnpm-lock.yaml` and root `package.json` are parent-owned. The per-app dependency manifest alone is not evidence of a reproducible build. `bundle.active=false`: **no installer is produced** at M1-C.

## M1-D/E/F gates

Integrate authenticated pairing, local Windows consent UI with no remote approval RPC, private-interface discovery, crypto session binding, bounded input policy and real native mouse/keyboard release; then package/install and verify with physical Windows 11/Android devices. A simulated, isolated or headless unit test is not native E2E acceptance.
