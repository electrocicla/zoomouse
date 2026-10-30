# Zoomouse R6-HOST — Rust trusted ingress safety seam

**Task:** `deployment-child-4ac8028860e06e53217fc42019b7c5b1`; parent `zoomouse-hourly-20261030-r6-7b42`.
**Repository:** `github.com/electrocicla/zoomouse`, branch `main`, verified baseline `ac474bd1a3f0` tracking `origin/main` ahead 0 / behind 0 (2026-10-30).
**Ownership:** exclusively `crates/host/**` and this new `workplan/workers/R6-HOST.md`. No edits outside the owned paths.

## Analysis and plan (before edits)

Reviewed `docs/{ARCHITECTURE,ROADMAP,QA-2026-10-30,SECURITY}.md`, `workplan/{README,PLAN,TASKS,WORKLOG,ACCEPTANCE-M1}.md`, `crates/host/{Cargo.toml,src/{lib,admission,boundary,consent,pairing,pairing_tcp,lan,input,error}.rs}`; read-only `crates/control/src/policy.rs` and `packages/protocol/src/pairing-wire.ts`. The previous `HostAdmission` lacked production constructors for encrypted channel and native pairing/session approval receipts; Tauri uses `DisabledInput`, TCP adapter is inactive and plaintext, and no real encrypted DataChannel or OS-local consent callback is installed.

Plan: (1) introduce private transactional frame-sequence/size/action/rate checks from a future trusted decoder; (2) feed only the exact native-evidence channel into `HostAdmission`, disconnect by whole channel identity instead of an unauthenticated string; (3) adversarial Rust tests for replay, foreign device, invalid input, revocation and release; (4) validate scoped Cargo and publish owned paths; (5) never enable networking/input or report physical acceptance.

## Scoped implementation

- **NEW** `crates/host/src/ingress.rs`: pure `IngressGate` with exact decoder-reported 1..4096 encoded bytes, first sequence=1, monotonic sequence bounded jump <=64, monotonic local `Instant`, rates <=90 events/sec, <=60 pointer moves/sec, <=5 text events/sec; finite bounded motion/scroll, bounded text and conservative key code. Transactional checks cannot consume sequence on failure.
- `crates/host/src/admission.rs`: private `VerifiedInputFrame` (no constructors), `dispatch_verified_frame` validates channel and ingress policy before any backend call; violation triggers revocation and unconditional held-input release. Bare input dispatch is test-only. Exact authenticated channel identity required for close notification; mismatched close cannot revoke another device. Ingress state clears upon revoke/stop/disconnect and starts anew after separately proven native session approval.
- `crates/host/src/error.rs`: explicit `InvalidFrame` error.
- `crates/host/src/lib.rs`: private ingress module.
- Added negative-path tests in ingress/admission modules. Existing pairing and approval constructors still cannot be reached from untrusted input.

## OS authority and live encrypted transport: intentionally NOT activated

Windows session observation can be built from **trusted Win32 calls**, not renderer IPC: `ProcessIdToSessionId`, `WTSQuerySessionInformationW`, `WTSRegisterSessionNotification` / `WM_WTSSESSION_CHANGE` for lock/disconnect (plus initial state checks), and `OpenInputDesktop` / `GetUserObjectInformationW(UOI_NAME)` to verify the active interactive desktop and deny Winlogon/secure desktop. Native approval requires a host-owned Win32 interactive-user-session dialog callback, bound to a specific native pending request and re-checked OS state. All state transitions fail closed on unknown/error/desktop change. These APIs **were researched but are not implemented in this task**: current host forbids unsafe Rust and has no reviewed OS adapter/native callback. Win32 dialog render, WTS session event integration, lifetime/identity proof, Session0 guard and lock/UAC physical tests remain prerequisites.

A vetted WebRTC backend must expose an **authenticated DTLS channel peer certificate/fingerprint and actual DataChannel binding**, compare this with the locally enrolled Ed25519 device identity through a correctly specified cryptographic protocol, enforce session freshness, decoder-reported lengths/timestamps and trusted monotonic sequence origin. An mDNS/QR hint, pairing Ed25519 signature, plaintext pairing TCP or Tauri IPC must never construct `VerifiedChannelEvidence`. This task adds neither a WebRTC dependency nor a listener, socket, firewall exception, native input injection, or network production constructor. `BootstrapStatus::DisabledMissingNativeAuthority` remains mandatory.

## Validation / publication

- Initial Cargo submission at 23:28 UTC **rejected before execution** because WIN claimed `apps/windows/src-tauri/src/lib.rs`; second at 23:31 rejected before execution because ANDROID claimed `apps/mobile/src/domain/lan-attempt.test.ts`. Later workspace reservations caused other pre-execution denials; all peers were notified and no ungoverned bypass was used.
- First governed `cargo fmt --manifest-path crates/host/Cargo.toml` **PASS exit0**, `termrun_fe772bc00f5446278a6e2ab9791c3110`. First `cargo check` **PASS exit0**, `termrun_35a60dfec6de4446ac3bd39a5b69079e`.
- First `cargo test` **FAIL exit101**, `termrun_0bea1c57616b4cd8a9e2dea5a0c0630f`: 10 E0616 test-only access errors because tests in `admission.rs` accessed private `boundary::HostSession.backend`. Fixed by introducing `#[cfg(test)] pub(crate) fn test_backend(&self) -> &B` in owned `crates/host/src/boundary.rs` and updating admission test assertions. There is no production backend getter.
- Rerun governed `cargo fmt --manifest-path crates/host/Cargo.toml` **PASS exit0**, `termrun_b46e64992c034818a4e2a6fc4f68a78f`.
- Rerun governed `cargo test --manifest-path crates/host/Cargo.toml` **PASS exit0**, `termrun_d431885f73b14599a2655a59f54bd2bc`, **48 unit tests + 1 doctest**, 0 failures.
- `cargo clippy --manifest-path crates/host/Cargo.toml --all-targets -- -D warnings` **PASS exit0**, `termrun_074027d53c3b47a1b8491ee68d6d723c`.
- `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` **PASS exit0**, `termrun_13016231145644d8b17caf36b65b1f18`.
- Final `cargo check --manifest-path crates/host/Cargo.toml` **PASS exit0**, `termrun_861c871130db4566aed99fc9bfb1a942`.
- Governed Git publication of **six** owned files: pending separate stage/commit/push receipts. No unauthorized path was staged.
- M1 physical A01–A17: **NOT RUN**. No APK, installer, OS-local approval, authenticated DataChannel, native input or captured video was accepted. No M1 release readiness claim.

## Remaining gates

1. Publish only the six owned files to `main` using governed Git FIFO; verify `origin/main` ahead=0 behind=0.
2. Parent to coordinate a reviewed Windows OS status/consent native library, live encrypted DataChannel identity/binding API, host+control cross-crate wiring, firewall tests, and physical A01–A17 acceptance. No test fixture counts as native authority.
