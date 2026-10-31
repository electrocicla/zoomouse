# Zoomouse M2-B — native GPU crop/H.264 pipeline foundation

## Identity and scope
- Child: `deployment-child-c2e1fd2c6611abc865ec216c3cc954e1`; instance: `web-child-c2e1fd2c6611abc865ec216c3cc954e1`; lifecycle: `2799a1d2-3ab7-4912-afc2-29c67406b20f`.
- Parent: `zoomouse-hourly-20261030-orchestrator-r2-explicit`.
- Repository `github.com/electrocicla/zoomouse`, authorized root 13, branch `main`, verified base `f01cb6cc8e42`, upstream initially 0/0.
- Exclusive paths: NEW `crates/media/**` and this file only. No integration into root Cargo, host, capture, Tauri, protocol or Android while those are owned by peers.

## Source and threat model reviewed
Read `docs/{ARCHITECTURE,ROADMAP}.md`, `workplan/{PLAN,TASKS,ACCEPTANCE-M1}.md`, all `crates/capture` source and README, Android M1-B README/transport/session, M2-A worklog and host lib.
WGC M2-A emits `GpuFrame {metadata, surface}` keeping its WGC `Direct3D11CaptureFrame` alive. Metadata: locally picked `SourceId`, grant generation, strictly increasing local sequence, QPC-relative timestamp in 100 ns, dynamic content size; WGC frame pool creates BGRA8 UNORM GPU surfaces. Native caller must synchronously GPU-copy before dropping the owned WGC frame. Capture authority is only a trusted-host trait (default DenyAll), not cryptographic consent itself.
Mobile native video rendering/WebRTC not implemented; no approved network media transport exists. Windows DXGI device selection, format/color interpretation (SDR/HDR), crop shaders/video processor, NV12 color conversion, MF hardware MFT and ownership synchronization are unresolved. Protected content/UAC/Session0 are excluded.

## High-level plan
1. Create standalone Rust `crates/media` crate with independent Cargo manifest and typed capture metadata contract.
2. Validate device/session/generation/source and trusted host authorization on ingest and before output; reject replay, age/future timestamp, resize/revocation/unknown formats.
3. Pure bounded geometry/pixel format/profile/timestamp/backpressure contracts, tests including negatives. No CPU texture transfer, network or remote authority inputs.
4. Provide Windows `GpuFrame` adapter and fail-closed unavailable D3D11 crop/scale + MF H.264 backend until native frame/device/encoder runtime is validated.
5. Run scoped Rust fmt/test/check/clippy through governed Windows terminal when workspace claim admission permits; then owned-only Git FIFO publication, parent notification.

## Coordination / blockers observed
- Parent Git FIFO workspace claim blocked initial `crates/media/Cargo.toml` guarded creation BEFORE any mutation; after release, gateway returned `parent_path_not_found` because `crates/media` does not exist.
- Governed terminal create succeeded; initial scoped directory command rejected due to live M1-D-LIVE claim. Retried only after shared reservation release: terminal run `termrun_fc3425a553be41159e43f96b51335216` exit 0. Exact owner-only files created under `crates/media/**`. All five media file claims were released before Cargo validation; peers M1-D-LIVE and M1-E were directly notified; native QA serialized through host workspace operations.
- Initial parent message delivery failed global unread higher-priority retention saturation. Later parent session was no longer active, so direct updates could not be delivered; semantic Agents Global logs are canonical parent evidence. Critical collision notices from M1-D-LIVE/M1-E were consumed, replied to and acknowledged.

## Checks / receipts
- **PASS** host-tracked Windows x86_64 Cargo test `termrun_fa8a1a8cf9f04d3aaa0d7dc59b6c7f7d`, command `cargo test --manifest-path crates/media/Cargo.toml`, exit **0**, 8 unit tests passed, 0 doc tests. Compiled Windows native WGC/GpuFrame borrowing seam and fail-closed unavailable adapter, not a real codec.
- **PASS** host-tracked rustfmt `termrun_aba6de18835b47c4b2994112c32eb4e1`, `cargo fmt --manifest-path crates/media/Cargo.toml`, exit **0**, owned Rust sources formatted.
- **PASS** host-tracked `cargo check --manifest-path crates/media/Cargo.toml` `termrun_254a416533da4b9197850f0f735dbd6b` exit **0**, compiled media + WGC on Windows.
- First strict Clippy `termrun_07104e62758545e381a1d0b2b54a5ddd` failed exit 101 on 8 `manual_is_multiple_of` warnings (`-D warnings`). Exact source-only fix replaced divisibility checks with `is_multiple_of(2)`. An initial broad CAS replacement temporarily generated invalid `output.!width` syntax; `cargo fmt -- --check` `termrun_e564c90054aa4e4cb70eeea4050aa052` detected and failed exit1. A second scoped CAS corrected all 4 malformed expressions. Final host-tracked `cargo fmt --manifest-path crates/media/Cargo.toml` run `termrun_e2aa98e123c14d85a40f1e42c75a819f` **exit0**, strict `cargo clippy --manifest-path crates/media/Cargo.toml --all-targets -- -D warnings` `termrun_e8f77f6e658f4875b2a9d8fc9c942e76` **exit0**, and `cargo test --manifest-path crates/media/Cargo.toml` `termrun_e1bbf73d33144efbb2263e487a286863` **exit0 (8/8 unit tests, 0 doc tests)**, all after final correction. Earlier `cargo check` `termrun_254a416533da4b9197850f0f735dbd6b` was exit0 before last style change; Clippy compiled current final source with warnings denied. Physical M2 acceptance remains NOT RUN.
- Physical Windows capture, D3D11 crop/scale, H.264 hardware MFT, authenticated WebRTC, Android playback, SDR/HDR color/rotation, GPU device loss, latency/resource measurements: **NOT RUN**. No media frames were produced or sent.

## Publication
- Owned source list: `crates/media/{.gitignore,Cargo.toml,Cargo.lock,README.md,src/lib.rs,src/native.rs}` and `workplan/workers/M2-B.md`.
- **Source commit** `ab11b160faf0` (`feat(media): add fail-closed Windows media admission foundation`) contained exactly 7 owned paths; Git FIFO stage/commit succeeded. `notcode_git_push(remote=origin,branch=main,force=false)` confirmed update `5f72298` -> `ab11b16` on `github.com:electrocicla/zoomouse.git`. Follow-up `notcode_git_summary` reported `branch=main`, `tracking=origin/main`, `ahead=0`, `behind=0`, `HEAD=ab11b160faf0`, owned files clean; 76 unrelated dirty paths remained untouched. This worker worklog closure line is a post-publication documentation-only change to publish in a separate scoped commit; no source or physical acceptance claim changed.
- Parent integration: add dependency to host by authorized host owner; bind current `CaptureScope` to trusted paired device, grant generation and OS-pick source; own D3D11 interop/texture pool and MF H.264 MFT; recheck revoked state post-encode and pre-WebRTC send; integrate real Android decoder and run M1-F/M2 physical acceptance.

## Independent R8 receipts recovery (2026-10-30)
- Current `main` history independently includes source commit `ab11b160faf0955a8c6742e7522ded338cacbc7a` as an ancestor (not merely a worker assertion); the existing source publication receipt above remains valid. Inspected current `crates/media/{Cargo.toml,README.md,src/lib.rs,src/native.rs}`: 8 Rust unit test declarations, deny-by-default admission, and Windows native adapter that returns `BackendUnavailable` without encoding/transmitting media.
- Original eight-test/fmt/check/clippy run IDs above are historical worker receipts. Fresh scoped `cargo fmt --manifest-path crates/media/Cargo.toml -- --check` was **not executed**: governed terminal reservation was rejected due to active R8-PROTOCOL file claim; no test result or exit code may be inferred. A fresh full Cargo QA receipt remains pending FIFO admission. No Windows video, WebRTC stream, GPU crop/encoder, physical M2 or A01–A17 evidence exists.

