# R8-HOST-TRUST — Windows consent/session provenance hardening

**Task:** `deployment-child-ba7eb7eeecffa99c05d863cc7fa528d7` · Parent `zoomouse-r8-orch-20261030-2120` · Connector `MK059` · authorized rootIndex 13 `Zoomouse`.

## Baseline and bounded scope

- 2026-10-30 21:25 CLST: `github.com/electrocicla/zoomouse`, branch `main`, `origin/main` ahead 0 / behind 0, HEAD `516c57c88712`.
- Foreign dirty/untracked paths were `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/M2-B.md`, `workplan/workers/R6-WIN.md` and `workplan/workers/M2-C.md`. No foreign edit, reset, stage or claim.
- Owned `crates/host/src/admission.rs` and this new worker record only. R8-PROTOCOL owns `packages/protocol/**`; R6-WIN and R6-ANDROID own their native app paths.
- Read workplan/README, PLAN, TASKS, WORKLOG, ACCEPTANCE-M1; docs/ARCHITECTURE, ROADMAP, SECURITY; host admission/boundary/channel_binding/consent/ingress/pairing/pairing_tcp/input/lan/error/lib and Cargo; Windows Tauri lib, Android lan-attempt, protocol fixture and Rust control executor.
- Threat model: renderer IPC/QR/plain TCP/test channel fields and JS hints must never issue native consent or authenticated DTLS authority; DesktopContext::from_local_os and both local approval receipt structs have no trusted production creator. Tauri currently exposes status/emergency revoke only; no OS-owned callback, desktop-token verification, DTLS exporter/stream producer or encrypted input decoder; Android native port is disabled.

## High-level implementation and invariants

1. Keep `BootstrapStatus::DisabledMissingNativeAuthority`, `DisabledInput`, no network listener or native injection.
2. In host admission, generate a 128-bit nonce via OS CSPRNG before opening a pending session-consent request; retain nonce, monotonic creation time and initiating Windows session privately alongside the pending channel. Callback must match nonce, exact authenticated channel/device, enrolled key, same OS-observed desktop, and arrive in under 60 seconds without clock rewind. Always consume the pending entry; failure revokes enrollment and releases input.
3. Add negative Rust tests for expired/replayed native decisions and nonce tampering/clock regression; preserve all earlier channel, input, consent and pairing tests. These tests use **fixture-only** mock desktop/native receipt and prove source-state-machine behavior, NOT Windows provenance.
4. Run authoritative scoped Cargo fmt/check/test/clippy and publish only owned files through Git FIFO once peer file claims and workspace validation lease permit it.

## Verification and release gates

- As of 2026-10-30 21:31 CLST, **Cargo NOT YET RUN**: the tracked terminal submit `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` was denied before execution with `Agents Global workspace cannot be reserved while R8-PROTOCOL owns packages/protocol/src/channel-proof-fixture.ts`. Peer notified; do not infer fmt/test success or re-run while claim active.
- No Windows OS interactive-user/session token inspection or trusted native consent UI callback is implemented; no native DTLS observation or enrolled Android secure channel; native input disabled, no WAN listener.
- Physical A01–A17, Windows installer, APK, capture and remote input: **NOT RUN**.
- Commit/push: **PENDING validation and governed Git FIFO**.

## Outstanding integration dependencies

A reviewed Win32/Tauri host-owned OS observation (current token/logon session, input desktop, lock/switch/UAC transitions) and unforgeable local callback in the same trusted process must back these pending records. A reviewed DTLS/SCTP native producer must bind both certificate pins, exporter, SCTP stream, host key and enrolled phone to this exact channel; Android must independently verify it. Consent token must never transit renderer IPC or packet content. Only then may production constructors and secure transport/injection gates be considered; separately require per-input desktop revalidation and reliable held-key release.

## Evidence checkpoint

- Startup selected verified Zoomouse root13, Agents Global session `e51a2c52-665c-44e6-9450-e00966b8f811`.
- Changed file: `crates/host/src/admission.rs` only; worker log newly created.
- Scoped source diff inspected; awaiting full scoped Cargo QA before any correctness or publication claim.

## Final validation receipt (supersedes earlier pending checkpoint)

- 2026-10-30 21:38 CLST, scoped Windows host Cargo on main source, bounded and tracked:
- `cargo fmt --manifest-path crates/host/Cargo.toml` PASS exit0, `termrun_d2ad31e94ff1448784303e1175583364` (applied Rust formatting).
- `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` PASS exit0, `termrun_e752befcf572406a815ae21edcdcef38`.
- `cargo check --manifest-path crates/host/Cargo.toml` PASS exit0, `termrun_38d42ac7028649dcaa5d8304c8b7bed6`.
- `cargo test --manifest-path crates/host/Cargo.toml` PASS exit0, `termrun_d18b7bb89db046c4b61fd3e3776bc9d5`; 56/56 unit + 1/1 doctest.
- `cargo clippy --manifest-path crates/host/Cargo.toml --all-targets -- -D warnings` PASS exit0, `termrun_0bb62776bbc947578b944ea793d7fded`.
- Earlier NOT-PASS attempts: workspace lease blocked by peer claims before execution; tracked initial `termrun_4065256d0c1d40b5ba91e989d11b337d` fmt parse failure due source tool read redaction accidentally copied into a pre-existing test expression (fixed immediately to a valid `other.public_key[0] ^= 0xff;`); Clippy `termrun_40a1f06ce54d4872a19df7b54bd7623a` failed on needless borrow (fixed). All final gates re-run and passed.
- Production authority remains UNIMPLEMENTED: Windows native OS callback and user-token provenance, authenticated DTLS/WebRTC transport and paired phone session, native injection and physical A01-A17 NOT RUN. No installer, APK or input enablement.
- Scoped source+worker log commit `f0ea8d29dbdf` (`fix(host): bind native consent callback to one-shot host nonce and deadline`) pushed to `origin/main`, verified ahead 0 / behind 0. This follow-up log correction records that exact durable source SHA. No foreign staged or committed paths.
