# M2-A — Windows Graphics Capture foundation (independent worker)

## Task and ownership
- Assigned childTaskId: `deployment-child-4536a5890b6d1a47e477248ab42a9cd0`
- Agent instance: `web-child-4536a5890b6d1a47e477248ab42a9cd0`
- Agent lifecycleEpoch: `03459b83-d036-4301-a680-ce8bfc469d5a`
- Parent: `zoomouse-orch-recovery-20261030`
- Repository: `github.com/electrocicla/zoomouse`; branch `main`; authorized rootIndex **13 / Zoomouse**, verified after select.
- Initial baseline: `d333dfe32f1a4b6eeaa1ee363c4063dbe7cc56b3`, `origin/main` synchronized. During execution other workers published protocol commits; current HEAD must be rechecked before publication.
- **Owned paths exclusively:** `crates/capture/**` and this new `workplan/workers/M2-A.md`. No root, host, Tauri, protocol, mobile or global workplan modifications.

## Analysis and implementation
Read `workplan/{PLAN,TASKS,WORKLOG,ACCEPTANCE-M1}.md`, `docs/{ARCHITECTURE,ROADMAP,QA-2026-10-30}.md`, Rust host/Tauri manifests/main and protocol capture schema before design. M1 physical gates remain NOT RUN.

New isolated `zoomouse-capture` Rust crate provides:
- `CaptureScope` device/session/grant generation identity validation; trusted `CaptureAuthorization` checks before local picker, after local picker, before WGC start and **each** frame poll. Built-in `DenyAll` means zero capture by default.
- Windows 11 `GraphicsCapturePicker` owned by local native `HWND`; source selection cannot be replaced with a remote-supplied HWND/HMONITOR. Opaque typed `SourceId` is informational; picker kind accurately tagged `UserPicked`.
- Real WGC `Direct3D11CaptureFramePool::CreateFreeThreaded`, `CreateCaptureSession`, `StartCapture`, `FrameArrived`/source `Closed` handlers, GPU-backed `IDirect3DSurface` frame, per-frame QPC-relative timestamp, dimensions and sequence; resize/recreate, close/revoke, cancellation.
- Pure desktop geometry: negative desktop origins, half-open bounds, 0/90/180/270 clockwise rotation, explicit per-monitor logical-to-physical DPI mapping; out-of-source cursor returns None; no implicit double DPI conversion.
- Root-level `crates/capture/README.md` documents integration/API trust boundaries and separate remaining native/physical acceptance gates.

### Cross-module integration required from parent/M2-B/C
Parent/host owner must add `zoomouse-capture = { path = "../capture" }` to `crates/host/Cargo.toml`, bind real authenticated session grant generation, native local source picker Tauri HWND, revoke during emergency-stop/disconnect/lock and add a persistent visual capture indicator. M2-B supplies the actual WinRT `IDirect3DDevice`, consumes the GPU surface, implements GPU crop/scale, bounded H.264 encoder and authenticated WebRTC transport; post-revocation in-flight frames must be discarded before encoding or sending. M2-C supplies verified physical desktop source bounds, rotation and monitor DPI and performs real cursor-follow/zoom QA. **None of these shared changes were made or claimed complete by M2-A.**

## Validation evidence (native Windows host, 2026-10-30 UTC)
- **PASS** tracked terminal `termrun_ab17c7697b554671802f4c282c7f0992`: PowerShell mkdir `crates/capture/src`, observed exit 0.
- **PASS** tracked terminal `termrun_aeb0e12e9d5b412fb2e89d35e7b1a68f`: `cargo test --manifest-path crates/capture/Cargo.toml`, observed exit **0** on Windows x86_64; **7 unit tests passed** (3 authority + 4 geometry) and 0 doc tests. This compiled the `cfg(windows)` native WGC module and windows v0.62.2 API surface. It did **not** start native capture or observe frames.
- **PASS** tracked terminal `termrun_bfa65f346db241f994e68fcd9b32bc56`: `cargo fmt --manifest-path crates/capture/Cargo.toml`, observed exit **0**. Formatted owned Rust sources.
- **PASS** tracked terminal `termrun_a46bf64f1ff344dda5f267d63c967f30`: `cargo clippy --manifest-path crates/capture/Cargo.toml --all-targets -- -D warnings`, observed exit **0** on Windows.
- Git publication: **PENDING**. Update only after exact staged file list, commit SHA and push/upstream receipts.

## Unfinished and blocked gates (not DONE)
- Default authority DENIES capture; real trusted host consent verifier, Tauri window binding and D3D11 device integration are not wired. No local OS picker opened, no physical WGC frame observed. Source cancellation and device-loss/runtime error paths have not had physical QA.
- No source/window-type classification from OS picker beyond `UserPicked`; source kind must not be guessed.
- No approved remote frame disclosure, stream state, network listener, crop, H.264, WebRTC, Android decode, cursor-follow, measurements or packaging. M2 real-device acceptance depends on M1-F and M2-B/C.
- Resource coordination: transient Agents Global workspace Git FIFO/file lease collisions with active M1 workers prevented some terminal commands **before execution**; released our exact temporary claims. No overlapping files were mutated. All checks above are only those with authoritative tracked run completion.
- Keep Windows yellow border default; in-process trait must not be mistaken for cryptographic approval. Protect outstanding GPU frames on revoke via M2-B output gate.

## Completion checklist
- [x] Agent instructions / workspace scope + connect
- [x] Read plan/security/contracts and Git/peer claims
- [x] New standalone crate with native WGC source, scope gate and geometry tests
- [x] Windows Cargo tests, rustfmt and strict Clippy
- [ ] Scoped Git stage/commit/push, verified upstream receipt
- [ ] Parent integration / physical WGC device+frame acceptance (external dependency)
