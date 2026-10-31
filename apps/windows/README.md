# Zoomouse Windows 11 — M1-C user-session tray/settings scaffold

`apps/windows` contains an independent Tauri 2 + React/TypeScript desktop project with a system-tray icon generated in memory, local session status and a one-way emergency-stop/revoke action.

The native app owns a `Mutex<HostSession<DisabledInput>>`. Its Tauri commands are deliberately limited to `host_status` and `emergency_stop`. There is **no** approve, remote network, shell, filesystem or process-execution command, no elevation, Session 0 service, Win32 input injection or screen capture. The UI explicitly labels LAN pairing, remote input and capture as **not implemented**. The emergency-stop action denies further input and invokes backend cleanup. The native shell currently has no OS-observed `DesktopContext`; its user-session-verification flag intentionally stays false, and the renderer labels the session scope **Unverified** until this trusted host integration exists. No interactive desktop state or consent is fabricated. Native-status IPC is checked at runtime; malformed, stale or unavailable state is displayed as **Unverified**, not as an invented safe/approved state. Tray Revoke exposes the settings window when release cleanup fails, while tray Quit refuses silent exit on cleanup failure. Closing the settings window hides it to tray; Quit is an explicit tray-only action.

## Development prerequisites / commands

Windows 11 interactive user account, Rust MSVC toolchain, C++ build tools, WebView2, pnpm 10 and Tauri 2 prerequisites. After dependencies are installed: `pnpm --filter @zoomouse/windows test`, `pnpm --filter @zoomouse/windows typecheck`, `pnpm --filter @zoomouse/windows build`, and from `apps/windows` `pnpm tauri dev`. Node 22's experimental type-stripping is used for the zero-dependency status tests. Rust host pure tests: `cargo test --manifest-path crates/host/Cargo.toml`. Tauri Rust compile: `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml`.

M1-C QA at HEAD baseline `121429f1b97b`: the focused Windows tests (3/3), strict TypeScript, Vite frontend bundle and native `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` succeeded on the local Windows toolchain (see `workplan/workers/M1-C-QA.md` for tracked run IDs). Icons are generated from `src-tauri/icon.svg` with `pnpm --filter @zoomouse/windows exec tauri icon src-tauri/icon.svg`; Windows icon artifacts are checked in, while other-platform icons and generated Tauri schemas are ignored. A successful cargo **check** is not a launch, installation, release build or signature. The root `pnpm-lock.yaml` and root `package.json` are parent-owned. The per-app dependency manifest alone is not evidence of a reproducible build. `bundle.active=false`: **no installer is produced** at M1-C.

## R9-B Windows 11 native EXE source-readiness contract (2026-10-31)

**Artifact boundary.** This project uses Tauri **2**, with `bundle.active=false` in
`src-tauri/tauri.conf.json`, and Cargo `[[bin]] name = "zoomouse-desktop"`.
`pnpm --filter @zoomouse/windows exec tauri build --no-bundle` is the **owner-run**
Tauri command for compiling an unsigned, unbundled local-development executable
on a suitably provisioned Windows 11 x64 user session. The *expected output if
that command actually succeeds* is
`apps/windows/src-tauri/target/release/zoomouse-desktop.exe`. No such release
binary, verified execution or self-contained portable distribution is asserted
by this source-only handoff. Any generated `dist/` directory contains frontend output only.
The local EXE runs under the invoking interactive user; it is **not** an
installed Windows service or a functional LAN server.

**Prerequisites (owner provisioned, not installed by this worker).** Windows 11
interactive account; Visual Studio C++ Build Tools with MSVC toolset and Windows
SDK; Rust `stable-x86_64-pc-windows-msvc` toolchain and Cargo; Node 22+
and Corepack/pnpm 10 (repository pins pnpm 10.33.0); Tauri 2 CLI dependencies;
Microsoft Edge **WebView2 Runtime** on the machine where the app is launched.
A successful Rust `cargo check` proves type/link preparation, not a runnable
release executable. Dependency install must use the repository owner's
validated root lockfile, currently modified in a foreign dirty working tree.

**Read-only prerequisite inspection and safe source QA** (PowerShell at repo root;
no release build, installer, signing or native permission request):

```powershell
node --version
corepack pnpm --version
rustc --version
cargo --version
rustup show active-toolchain
rustup target list --installed
corepack pnpm --filter @zoomouse/windows test
corepack pnpm --filter @zoomouse/windows typecheck
corepack pnpm exec eslint apps/windows/src
cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check
cargo check --manifest-path apps/windows/src-tauri/Cargo.toml
cargo test --manifest-path apps/windows/src-tauri/Cargo.toml
cargo clippy --manifest-path apps/windows/src-tauri/Cargo.toml --all-targets -- -D warnings
```

These checks **do not** prove a raw release executable exists. The focused
`build-readiness.test.mjs` tests intentionally pin the no-bundle Tauri
configuration, Cargo executable name, minimal capabilities and denied native
approval surface; changing those contracts requires an explicit review.

**Separate owner-authorized native compile step (NOT RUN here):**

```powershell
# At repository root, only AFTER reconciling the root lockfile and toolchain:
corepack pnpm install --frozen-lockfile
corepack pnpm --filter @zoomouse/windows exec tauri build --no-bundle
Test-Path .\apps\windows\src-tauri\target\release\zoomouse-desktop.exe
```

Only a successful build **and** positive file verification establish the raw
EXE artifact; launch/manual-tray QA and host runtime dependency checks remain
separate. Do not treat the `Test-Path` example as an observed result. The
`pnpm tauri build` flow may invoke the configured frontend build; it is *not*
the same thing as the frontend-only `pnpm --filter @zoomouse/windows build`.

**Installer is a distinct, later explicit product decision.** An MSI or NSIS
`-setup.exe` requires an approved bundling configuration and a separate Tauri
bundle operation. This handoff deliberately does **not** toggle `bundle.active`,
choose an installer target, sign, install, request elevation or modify machine
firewall/permissions. A future per-user installer should explicitly retain the
NSIS `currentUser` install mode and be physically tested before distribution.
Local unsigned EXE development does not require Microsoft Store signing.
Neither unsigned compilation nor a per-user installer would authorize remote
input or remove the missing Windows OS-observed consent and authenticated
DTLS/channel proof gates. A01–A17 physical acceptance remains **NOT RUN**.

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
