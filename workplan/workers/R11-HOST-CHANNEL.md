# Zoomouse R11-HOST-CHANNEL — authenticated Rust host safety closeout (2026-11-04)

## Task and authorization
- Independent child task `deployment-child-a9def09068fe4d40b404367748dc2b59`, Agents Global lifecycle `1bc6697a-b866-4e54-94ed-909011f72641`; parent logical task `zoomouse-r11-orchestration-20261104`.
- Authorized workspace rootIndex **13**, **Zoomouse**, canonical `github.com/electrocicla/zoomouse`, branch `main`. Initial R11 docs HEAD `84ae2beb37c2`; upstream ahead0/behind0 at startup. Windows worker separately published `37d2e0ae7231` and its documentation `d551d3e71a62` before this host closeout.
- Exclusive write ownership: `crates/host/**` and this **new** `workplan/workers/R11-HOST-CHANNEL.md`. No edits to `apps/windows/**`, `apps/mobile/**`, `crates/control/**`, `packages/protocol/**` or shared parent plans.
- Four inherited unrelated dirty files **preserved, not staged, reset, or changed**: `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md`.

## Cascade review / security boundary
- Read `crates/host/src/{boundary,consent,admission,channel_binding,ingress,lan,pairing,pairing_tcp,lib,input}.rs`, `crates/control/src/lib.rs`, `apps/windows/src-tauri/src/lib.rs`, `packages/protocol/CHANNEL-PROOF.md`, R10/R11 execution plans and `workplan/TASKS.md`.
- `ObservedEncryptedChannel` / `ChannelProofChallenge` provide an Ed25519 proof substrate over exporter, certificate fingerprints, SCTP stream and host nonce **only when supplied by a genuine native DTLS/SCTP adapter**. This source has no such authenticated producer or Android equivalent. A signature from a synthetic fixture, QR metadata, TCP pairing, renderer or network callback is **not** verified native channel authority.
- `NativePairingApprovalReceipt`, `NativeApprovalReceipt` and `VerifiedChannelEvidence` deliberately have no production constructors; a physical OS-local Win11 user-session approval callback is not wired. `HostAdmission::bootstrap_status()` remains `DisabledMissingNativeAuthority` and `DisabledInput` cannot inject input. No public network listener, input, screen capture, or streaming enabled.
- Admission/ingress already include replay/session/nonce/TTL, lock/session-switch revocation, explicit channel disconnect and rate/sequence guards. Avoid duplicate or speculative native layers.

## Actual code corrections (SOURCE ONLY)
1. `crates/host/src/consent.rs`: a granted session now retains the local callback's monotonic `granted_at: Instant`. A frame timestamp **before** its grant (regressed clock) is denied as `HostError::Expired`, which routes through `HostSession::dispatch` revocation and native held-input release, just like an expired lease. Previously only the upper expiration boundary was checked.
2. `crates/host/src/boundary.rs`: `Drop for HostSession<B: InputBackend>` now best-effort calls `revoke()` / `release_all()`. Normal Rust host teardown no longer relies solely on callers remembering `disconnect()`. Destruction cannot guarantee release on process crash or failed native backend release; operating-system recovery remains a native integration requirement.
3. Added adversarial tests: `boundary::tests::rewound_grant_clock_revokes_and_releases_before_any_input` and `boundary::tests::dropping_approved_host_attempts_held_input_release`. Neither creates an authenticated transport, OS consent or production grant.

## Authoritative scoped Cargo validation (Windows PowerShell Not-Code terminal)
Terminal ID: `agent-term-1793815300911-3`; canonical repo root `I:\Zoomouse`. The following are **tracked host command runs with completionConfirmed=true**, not command-submission-only assertions.

| Command | Frozen runId | Exit / result |
|---|---|---|
| `cargo fmt --manifest-path crates/host/Cargo.toml --all -- --check` | `termrun_e894ad48193944fe8790ef17d16e719f` | **0**, format clean (18:06:52Z) |
| `cargo check --manifest-path crates/host/Cargo.toml --locked` | `termrun_dadf7ff2acc34e738c314ed15a82745f` | **0**, host crate compiled (18:07:11Z) |
| `cargo test --manifest-path crates/host/Cargo.toml --locked` | `termrun_c160854be87f4a5c8377e752817115f5` | **0**, **62/62 unit tests + 1/1 doctest** PASS, including both new negative tests (18:09:03Z) |
| `cargo clippy --manifest-path crates/host/Cargo.toml --locked --all-targets -- -D warnings` | `termrun_5be1a43bc5fb44c58e8ff265ffa3314e` | **0**, no warnings at strict threshold (18:18:19Z) |

Some workspace terminal submissions were **rejected before execution** by authenticated Agents Global parent Git FIFO/Android/Windows file claims; they are not failed test runs. Windows-first QA priority was coordinated through messages and the in-flight host test was reconciled by its exact run ID before yielding. Windows worker then confirmed all 8 scoped checks PASS, Windows source + doc commits pushed and released the FIFO, after which the single remaining host Clippy was run. Host file claims were released after source freeze; no competing source files modified.

## Acceptance and unfinished dependencies
- **HOST SOURCE QA: PASS** for the two bounded source corrections and existing host crate tests. The worker's exact-files commit/push SHA and upstream verification are communicated to the parent via the authoritative Agents Global completion report; this self-contained worker record does not invent its own yet-unknown SHA.
- **REAL NATIVE SECURITY INTEGRATION: BLOCKED / NOT ACCEPTED.** A reviewed native WebRTC/DTLS/SCTP host producer must independently observe same-association exporter + both certificate fingerprints + stream, bind verified enrolled Ed25519 phone identity, and interoperate with Android's authenticated native peer proof. A signed test transcript must never be elevated to trusted authority.
- **REAL OS-LOCAL APPROVAL: BLOCKED / NOT ACCEPTED.** A trusted Win11 session-specific callback after an actual physical choice must bind nonce/device/channel/deadline, recheck lock/UAC/desktop/user-switch/transport loss, and reject renderer/IPC/network synthetic callbacks. Native held keys/buttons must release under real lock/revoke/transport drop; failed release remains fail-closed.
- **PHYSICAL A01–A17: NOT RUN.** Neither Win11 physical runtime nor Android native E2E was run. No EXE/APK release build, packaging, install, elevation, signature or native firewall operation was performed.
- Final status is **SOURCE DONE FOR THIS BOUNDED HOST PATCH; NATIVE E2E NOT ACCEPTED**, not overall R11 or M1 security acceptance.
