# R11-M2-MEDIA — isolated native GPU-surface admission repair

## Identity, repository and ownership
- Task `deployment-child-e4f148a172c4dff711af8cf0486e4694`, agent `web-child-e4f148a172c4dff711af8cf0486e4694`, lifecycleEpoch `a313d5b5-8c34-4e06-ae92-c7cd3c5a0d6d`, parent `zoomouse-r11-orchestration-20261104`.
- On 2026-11-04 verified authorized rootIndex13 `Zoomouse`, canonical `github.com/electrocicla/zoomouse`, `main`, source HEAD `84ae2beb37c2`, tracking `origin/main` 0 ahead/0 behind.
- Exclusive edits: `crates/media/**`, this new `workplan/workers/R11-M2-MEDIA.md`. No `apps/windows/**`, `apps/mobile/**`, `crates/host/**`, `crates/capture/**`, `crates/viewport/**`, root Cargo/protocol/shared workplan mutations.
- Four inherited owner dirties `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md` left untouched/un-staged. Concurrent Android, Windows and host agent changes also foreign and untouched.

## Cascaded architecture/root-cause review
- `crates/capture/src/native.rs` produces a borrowed GPU-backed WGC `GpuFrame` owning the `Direct3D11CaptureFrame`; `surface` is a WinRT `IDirect3DSurface` and metadata includes locally minted source ID, grant generation, monotonic source-local sequence and QPC-relative timestamp. A capture-consent trait defaults to `DenyAll`. The WGC frame/pool is not attached to a production OS-local grant yet.
- `crates/media/src/lib.rs` enforces source, generation, metadata age/replay, BGRA8/declared SDR-BT709 limited, even crop, profile and 1–3 in-flight leases, but the public fields of `Size`, `CropPlan`, `EncodeProfile` previously permitted direct invalid construction and `MediaSession::new` did not independently revalidate all budgets. The old Windows adapter trusted a synthetic BGRA8 metadata claim without consulting the actual WinRT surface description.
- `crates/viewport/**` computes pure source-local even crop geometry, not real capture/authority. `crates/host/**` requires authenticated transport evidence plus native Windows session and local approval, not yet established for media. Tauri/Android paths do not expose a granted pipeline or sender.
- A real GPU crop/scale, color-managed BGRA8→NV12 path and D3D11-aware MF hardware MFT require device identity and native D3D11/DXGI interop, resource pooling/fences, verified SDR range, protected-content/HDCP policy, real hardware enumeration/encoder and OS consent. No proof those exist or work in this workspace. Do **not** fabricate an encoder, CPU fallback, remote sender or positive runtime authority.

## Scoped implementation
1. Revalidate all public `Size` and `EncodeProfile` fields upon creating a `MediaSession`, and both `CropPlan` source and output sizes even if callers bypass constructors. Preserve original max-source, max-output, FPS, bitrate, chroma and aspect budgets.
2. Add a real Windows WinRT `IDirect3DSurface.Description()` read in `UnavailableD3d11MediaFoundation::submit` while its borrowed WGC `GpuFrame` is alive **after authenticated admission**. Reject malformed/oversized dimensions, mismatch against native WGC frame metadata, non-BGRA8 formats (including HDR/unknown), and multisampled surfaces. WinRT/native surface error (including device loss) maps to explicit closed `BackendUnavailable`. Recheck revocation after native read. No native pixels are read/copied, no encoding and no external frame output.
3. Add negative Windows synthetic WinRT surface-descriptor tests and metadata-forgery admission test. No live video/OS picker is started.

## Validation and publication
Windows PowerShell from `I:\\Zoomouse`, host-tracked completion markers, all source-only:
- **PASS** `cargo fmt --manifest-path crates/media/Cargo.toml`: `termrun_fb0e6c37af6244818ed3902013bfb092`, exit **0**, changed owned native formatting only.
- **PASS** `cargo fmt --manifest-path crates/media/Cargo.toml -- --check`: `termrun_c8ee1d0bf02a41a8a73f31fee4925497`, exit **0**.
- **PASS** `cargo check --manifest-path crates/media/Cargo.toml`: `termrun_718a3d350b9c48c89cdf01e4bbd5bb4b`, exit **0**, verified native Windows `IDirect3DSurface.Description()` WinRT 0.62.2 binding compiles.
- **PASS** `cargo test --manifest-path crates/media/Cargo.toml`: `termrun_c770b4352ab24ea2b3e469eddfbf8a9b`, exit **0**, **12/12** unit tests (includes 3 Windows synthetic surface descriptors + 1 malformed public metadata), **0** doctests.
- **PASS** `cargo clippy --manifest-path crates/media/Cargo.toml --all-targets -- -D warnings`: `termrun_dea8887853464f3dbb1b686e6f25c828`, exit **0**, strict warning-free Windows native target.
- The initial attempted terminal read-only probe and earlier `cargo fmt -- --check` were **blocked before command execution** by other agents' workspace reservations; they have no run ID and are NOT failed tests. After all five media claims were released, R12 parent serialized Windows→Host→Media, allowing these five actual tracked PASS receipts.
- `Cargo.lock` changed due to the direct Windows `windows` dependency; it is an owned media-crate lockfile.
- Git source commit/push receipts: **PENDING at this checkpoint**. Stage only `crates/media/{Cargo.toml,Cargo.lock,README.md,src/lib.rs,src/native.rs}` and this worker note. Never stage unrelated owner or concurrent worker paths.

## Remaining blocked physical/native gates (not accepted)
- Real OS-local consent to capture a specifically selected Windows source; real WGC frame acquisition and GPU texture lifetime/provenance; D3D11 device/texture descriptor access, same-device handoff, crop/scale shader/video processor, GPU BGRA8→NV12 with validated SDR color range; Media Foundation H.264 hardware MFT enumeration/configuration, surface ownership and session-specific flush/revocation; GPU device loss; HDCP/protected content; unsupported HDR/multimonitor; producer timing; authenticated WebRTC; Android decoding and cursor-follow; latency/CPU/GPU budgets, physical M2 and A01–A17 E2E.
- No local EXE/APK package, signing, installation or unapproved screen capture was performed. **SOURCE scope only**, M2 native/physical acceptance **NOT ACCEPTED**.
