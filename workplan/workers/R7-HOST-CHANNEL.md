# Zoomouse R7-HOST-CHANNEL — encrypted transport proof precondition

**Child task:** `deployment-child-678ea0d3fff84aff6093f5189a532148`; **parent:** `zoomouse-r7-orch-20261030-2047`; **selected connector:** MK059.
**Repository:** `github.com/electrocicla/zoomouse`, branch `main`, initial HEAD `be0014b8769c`, `origin/main` ahead 0, behind 0.
**Authorized root:** Zoomouse rootIndex 13, Agents Global session/lifecycle `ea519bee-506d-48cd-a149-334c5189c7c3`.
**Exclusive file ownership:** `crates/host/**`, NEW `workplan/workers/R7-HOST-CHANNEL.md`. Parent owns shared workplan/root. Existing M2-B, R6-WIN and M2-C worker logs and root configuration are foreign dirty work; preserve.

## Pre-edit analysis / plan

Read `docs/{ARCHITECTURE,SECURITY,ROADMAP}.md`, `workplan/{README,PLAN,TASKS,ACCEPTANCE-M1}.md`, previous `workplan/workers/R6-HOST.md`, `crates/host/src/{lib,admission,ingress,boundary,consent,pairing,pairing_tcp,lan,input}.rs`, `packages/protocol/src/pairing-wire.ts`, and read-only `crates/control/src/native.rs`.
R6-HOST published guarded ingress and exact channel teardown (`a24e5488bf31` source / `05cef9681096` receipt); no WebRTC DTLS producer, binding certificate proof, OS-observed consent callback or native host→control connection. Existing `VerifiedChannelEvidence` and `NativeApprovalReceipt` deliberately have no production constructors; `HostAdmission::bootstrap_status()` returns `DisabledMissingNativeAuthority`.

Scoped plan: (1) implement ONLY an internal bounded one-shot cryptographic proof precondition over *backend-observed* DTLS exporter, both negotiated certificate SHA-256 fingerprints, negotiated SCTP stream ID, pinned host pairing key, enrolled Ed25519 phone key and CSPRNG host nonce; (2) adversarial tests for identity substitution, replay, certificate/exporter/stream modification, expiry and malformed observations; (3) no authority object, public API, plaintext listener, native input or UI callback; (4) Cargo fmt/check/test/clippy; (5) stage/commit/push ONLY owned files after Git FIFO and report to parent.

## Implementation checkpoint

- NEW `crates/host/src/channel_binding.rs`: internal constant-size challenge with SHA-256 domain-separated transcript, OS entropy, a single Ed25519 verification attempt and 20-second monotonic deadline. A valid proof is **not** a session or channel authority receipt. Android protocol support and live transport provenance are still unimplemented and require separate review.
- `crates/host/src/lib.rs`: private `channel_binding` module declaration; no exported constructors or activation path.
- Production policy unchanged: `BootstrapStatus::DisabledMissingNativeAuthority`, `DisabledInput`, no network socket/OS-local native UI grant.
- No tests may be described as physical LAN acceptance. A01–A17 all **NOT RUN**.

## QA / publication checkpoint

- Initial attempted governed terminal command `cargo fmt --manifest-path crates/host/Cargo.toml` was **rejected before execution** by Agents Global workspace reservation, because R7 parent task held `workplan/WORKLOG.md`. This is not a Cargo pass/failure. Sent high-priority coordination message to parent; do not bypass claims.
- Parent WORKLOG claim was later released and the scoped governed Cargo commands ran successfully; no bypass.
- `cargo fmt --manifest-path crates/host/Cargo.toml` PASS, exit 0, `termrun_85c0eab5585f42fba1a4c6f6de1bebb5`.
- First `cargo check --manifest-path crates/host/Cargo.toml` PASS, exit 0, `termrun_f4fdfb6902aa446bbbcf1eb87546ce3e`.
- First `cargo test --manifest-path crates/host/Cargo.toml` FAIL, exit 101, `termrun_05be3b3eb4d0416298ad525fca25f6c2`: 53 passed / 1 failed. Test incorrectly treated byte sequence `[0xff;32]` as necessarily invalid Ed25519 compressed public key; the Dalek decoder accepts it. Removed that invalid assumption while retaining explicit zero-value, wrong signer, expired/replay and substituted DTLS proof negatives; strengthened final signature check using Ed25519 `verify_strict`.
- Final `cargo test --manifest-path crates/host/Cargo.toml` **PASS exit 0**, `termrun_0286333fe5174a0fadfe4876c9a82332`: **54 unit tests and 1 compile-fail doctest**, 0 failed.
- `cargo clippy --manifest-path crates/host/Cargo.toml --all-targets -- -D warnings` **PASS exit 0**, `termrun_7b7ada8cc9bd444cb195e88f7906de64`.
- `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` **PASS exit 0**, `termrun_c1f13ad0af65450b875e9f66b37d0fb6`.
- Final `cargo check --manifest-path crates/host/Cargo.toml` **PASS exit 0**, `termrun_87141c847486481bba7682fcb5f0b8d3`.
- Environment: governed visible PowerShell on Windows, host path `I:\\Zoomouse`, crate `zoomouse-host 0.1.0`; test and clippy evidence is local source-only, not Android or native runtime certification.
- Production still **BLOCKED**: missing real reviewed WebRTC/DTLS exporter/certificate trusted producer, Android protocol-equivalent signature implementation, Tauri-independent Win11 OS-local consent callback, approved native host-control integration, secure desktop release tests, physical devices and A01–A17. Proof verifier is crate-private and is not wired to `HostAdmission`, by design.
- Scoped Git publication and upstream parity: **PENDING** until owned stage/commit/push receipt.
