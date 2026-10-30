# Zoomouse Windows 11 — M1-C user-session tray/settings scaffold

`apps/windows` contains an independent Tauri 2 + React/TypeScript desktop project with a system-tray icon generated in memory, local session status and a one-way emergency-stop/revoke action.

The native app owns a `Mutex<HostSession<DisabledInput>>`. Its Tauri commands are deliberately limited to `host_status` and `emergency_stop`. There is **no** approve, remote network, shell, filesystem or process-execution command, no elevation, Session 0 service, Win32 input injection or screen capture. The UI explicitly labels LAN pairing, remote input and capture as **not implemented**. The emergency-stop action denies further input and invokes backend cleanup. The native shell currently has no OS-observed `DesktopContext`; its user-session-verification flag intentionally stays false, and the renderer labels the session scope **Unverified** until this trusted host integration exists. No interactive desktop state or consent is fabricated. Native-status IPC is checked at runtime; malformed, stale or unavailable state is displayed as **Unverified**, not as an invented safe/approved state. Tray Revoke exposes the settings window when release cleanup fails, while tray Quit refuses silent exit on cleanup failure. Closing the settings window hides it to tray; Quit is an explicit tray-only action.

## Development prerequisites / commands

Windows 11 interactive user account, Rust MSVC toolchain, C++ build tools, WebView2, pnpm 10 and Tauri 2 prerequisites. After dependencies are installed: `pnpm --filter @zoomouse/windows test`, `pnpm --filter @zoomouse/windows typecheck`, `pnpm --filter @zoomouse/windows build`, and from `apps/windows` `pnpm tauri dev`. Node 22's experimental type-stripping is used for the zero-dependency status tests. Rust host pure tests: `cargo test --manifest-path crates/host/Cargo.toml`. Tauri Rust compile: `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml`.

M1-C QA at HEAD baseline `121429f1b97b`: the focused Windows tests (3/3), strict TypeScript, Vite frontend bundle and native `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` succeeded on the local Windows toolchain (see `workplan/workers/M1-C-QA.md` for tracked run IDs). Icons are generated from `src-tauri/icon.svg` with `pnpm --filter @zoomouse/windows exec tauri icon src-tauri/icon.svg`; Windows icon artifacts are checked in, while other-platform icons and generated Tauri schemas are ignored. A successful cargo **check** is not a launch, installation, release build or signature. The root `pnpm-lock.yaml` and root `package.json` are parent-owned. The per-app dependency manifest alone is not evidence of a reproducible build. `bundle.active=false`: **no installer is produced** at M1-C.

## M1-D/E/F gates

Integrate authenticated pairing, local Windows consent UI with no remote approval RPC, private-interface discovery, crypto session binding, bounded input policy and real native mouse/keyboard release; then package/install and verify with physical Windows 11/Android devices. A simulated, isolated or headless unit test is not native E2E acceptance.


## R6-WIN native authority diagnostics (source-only)

The Tauri Rust state now exposes **read-only** `approvalGate` and
`lastReleaseOutcome` in `host_status`. The fixed gate
`missingVerifiedChannelAndNativeApproval` explicitly means production
authorization is **not** available. The renderer rejects forged `Approved`,
`Pending`, active input, capture, discovery or OS-session claims while this
gate remains in effect. Its UX never offers an Approve button.

Emergency stop, tray Revoke and tray Quit use the same Rust-side revoke +
backend-release method. A release failure is latched for diagnostics and tray
Quit refuses to exit silently; tray Revoke surfaces the window on failure.
`confirmed` means **only** that `DisabledInput` returned success (not that
real Windows keys/buttons were exercised). A poisoned lock or unreadable IPC
remains `Unverified`; no cleanup guarantee is inferred.

The native authority contract needed before enabling an approval panel is:
(1) a DTLS/WebRTC adapter that proves the actual authenticated device public
key, session ID and channel binding; (2) OS-observed interactive-session
`DesktopContext`, including active/locked/secure-desktop changes; (3) a
host-owned native consent presentation tied to the exact pending
device/session/channel, where only an OS-local callback can mint
`NativePairingApprovalReceipt` and `NativeApprovalReceipt`; (4) expiry,
revoke/disconnect/desktop-loss cleanup with a real input backend; and (5)
approved private-LAN/firewall integration and A01–A17 physical tests.
`crates/host/src/admission.rs` exposes **no production constructors** for
those receipts/evidence, and its bootstrap is disabled. Do not add an IPC
command or renderer state that fabricates them. No installer is created by
this source change; user-owned packaging is separate.
