# R13-WINDOWS — independent native approval/input worker

## Scope and initial audit (2026-11-06)
- Task `deployment-child-7d75c97df8acc87b9f91bd6363b461d9`, instance `web-child-7d75c97df8acc87b9f91bd6363b461d9`, lifecycle `389e2fa2-2b6e-4737-a2ed-d57668b7742d`; Zoomouse rootIndex13, `github.com/electrocicla/zoomouse` `main`, HEAD `d851bb4ca5ac`, tracking `origin/main` ahead0/behind0 at start.
- Exclusive ownership: `apps/windows/**`, `crates/control/**`, this file. No `crates/host/**` or parent plans/TASKS/WORKLOG. Preserve inherited foreign dirty `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md`; inherited parent dirty TASKS and WORKLOG untouched.
- Traced `apps/windows/src-tauri/src/lib.rs` Tauri native tray, revoke, 500ms desktop observer, `desktop_observer.rs`; private `crates/control/src/executor.rs` → `native.rs` SendInput; `crates/host/src/{input,consent,boundary}.rs` read-only; `apps/windows/src/host-status.ts`; architecture/R11/R13 plans and A01–A17.
- Current native Tauri is intentionally `DisabledInput` with NO verified WebRTC/DTLS identity, peer-bound generation or OS-local approval constructor. React IPC and positive `DesktopObservation` cannot grant. Private `NativeInput` is not exported or wired to HostSession. Do not promote a synthetic native callback or test double into authority.

## Implementation plan BEFORE source edits
1. Harden Win32 desktop eligibility and teardown: exact bounded UOI_NAME, closing errors are denials, no desktop probe ever produces permission. Safely revoke/retain cleanup failure on unavailable desktop, lock/switch/UAC even when renderer hidden.
2. Harden isolated `crates/control` input executor/native release guards with adversarial failure and state tests; keep private SendInput unexported and disabled production dispatch. Review stale generation and revocation seam with Host worker without editing host.
3. Run tracked PowerShell Cargo fmt/check/test/clippy (affected crates), plus Windows TS tests/lint/typecheck if possible. No `tauri build`, release EXE, APK, signing, install, physical input or network listener.
4. Record exact tracked run IDs, command exit codes, observed tests, blockers, files, SHA and origin/main sync; stage/commit/push EXACT owned files after Git FIFO and snapshot. Native A01–A17 NOT RUN.

## Implemented security changes (source-only)
- `apps/windows/src-tauri/src/lib.rs`: native background desktop watcher revokes on loss (lock/UAC/console switch) and retries any failed held-key/button cleanup only on the next `Unavailable` → `InteractiveDefault` observation edge. Repeated locked or interactive samples cannot flood release; no prior consent is reinstated. Tauri status, emergency stop, tray quit and background observer salvage poisoned Rust `Mutex` ownership and attempt revoke/release rather than skipping cleanup. Tests cover unlock recovery, persistent release failures and poisoned mutex.
- `crates/control/src/native.rs`: private Windows SendInput adapter requires exact terminated `Default\0` UTF-16 bytes and exact UOI_NAME length; reject prefix, malformed, embedded-NUL, truncated, secure-desktop lookalikes, failed `CloseDesktop`, Session 0, RDP/console mismatch. No public constructor or dispatch entry point added.
- `crates/control/src/executor.rs`: negative tests for partial native key-down + failed cleanup (terminal state retained for later release retry), and last-resort Drop release of held key. No production caller wired. Host association proof (R13-HOST) is read-only, not modified.
- **Contract to HOST** (delivered via Agents Global): native-only authenticated DTLS/SCTP peer, session, association generation, live desktop and owner-local consent must precede dispatch; input requires per-event contemporaneous authority and synchronous revoke/release on user switch/lock/teardown. Never infer grant from renderer IPC, unit-test fixtures, QR, synthetic desktop probe or transport payload. Parent HOST-first Cargo/Git sequencing observed.

## Tracked Windows PowerShell source validation
All commands executed in visible agent-managed PowerShell, cwd `I:\\Zoomouse`, terminal `agent-term-1794001402647-24`. Run IDs are authoritative `notcode_terminal_send_line` → `notcode_terminal_read_safe(runId)` receipts with prompt-boundary `completionConfirmed=true`. Initial two attempts **before** host was released were rejected due foreign workspace reservations **without run IDs**; they are NOT represented as tests.

| Exact command | Tracked runId | Exit |
| --- | --- | ---: |
| `cargo fmt --manifest-path crates/control/Cargo.toml -- --check` (initial) | `termrun_79c4d0f5f0dd408d8c4252c7abcb45fa` | 1 (expected formatting diffs) |
| `cargo fmt --manifest-path crates/control/Cargo.toml` | `termrun_1a1f1bdb09fb4938b8b3324ae298a545` | 0 |
| `cargo fmt --manifest-path crates/control/Cargo.toml -- --check` (repeat) | `termrun_7ce3d59295664c5ea827bb60e30e0e2c` | 0 |
| `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check` (initial) | `termrun_b4638e77cbf441c2ae618bc69ec315d0` | 1 (expected formatting diffs) |
| `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_fcf08729508a4aecb3403d0b5da6d748` | 0 |
| `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check` (repeat) | `termrun_32ceca7a0aab433da233a8975d07e48a` | 0 |
| `cargo check --manifest-path crates/control/Cargo.toml` | `termrun_5da7e9cd4d9e40e9b4ad0fc75a967d9a` | 0 |
| `cargo test --manifest-path crates/control/Cargo.toml` | `termrun_ee5c68feb40a4e5e9b6469ff7aa6844f` | 0 — **10/10 unit**, 0 doctests |
| `cargo clippy --manifest-path crates/control/Cargo.toml --all-targets -- -D warnings` | `termrun_d95fb77a14644a0d84625966706502d3` | 0 |
| `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_5d3f421085764b449c72e7c110be5322` | 0 |
| `cargo test --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_b372264b738c4b6d8c468ef51f67868f` | 0 — **9/9 lib unit**, 0 bin/doctests |
| `cargo clippy --manifest-path apps/windows/src-tauri/Cargo.toml --all-targets -- -D warnings` | `termrun_e4ee8a99faac4b559278e68810d03edd` | 0 |
| `pnpm --filter @zoomouse/windows typecheck` | `termrun_ceb3bc087fa343ffaf7b8c13f872ed8c` | 0 — TypeScript `tsc --noEmit` |
| `pnpm --filter @zoomouse/windows test` | `termrun_84ce45fa5117486296de1d542f8456cb` | 0 — **9/9 Node unit** |

- `apps/windows/package.json` defines no separate `lint` script; did not fabricate a successful app linter. Rust `clippy -D warnings` and Windows TS `typecheck` executed as above. No `build`, native installer, process launch/approval, key injection, capture, network listener, APK, or package signing performed.
- Scope after native formatting: only `apps/windows/src-tauri/src/lib.rs`, `crates/control/src/native.rs`, `crates/control/src/executor.rs`, this `workplan/workers/R13-WINDOWS.md`; parent, HOST, Android and 4 protected inherited dirty paths untouched.
- HOST-FIRST serial QA respected: HOST published source `83bd4136eaf9` and evidence `5f2bedb70c8e` upstream ahead0/behind0 before our tracked Windows QA. Our own pre-Git HEAD `5f2bedb70c8e` origin/main 0/0.
- **Unaccepted gates:** no backend-observed real encrypted DataChannel, live Win11 OS-user-native approval UI, production `InputBackend` SendInput binding, physical lock/UAC/revoke proof, real Android/Win11 LAN input, WGC media/WebRTC, .exe/.apk, or owner physical A01–A17; pure unit tests are NOT device acceptance.

## Publication receipt
Owned source commit SHA and push receipt will be appended after scoped Git FIFO publication. Do not infer SHA or native acceptance from this pre-commit entry.
