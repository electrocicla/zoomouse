# Zoomouse — high-level implementation plan

**Product:** Android phone remotely controls an explicitly approved Windows 11 desktop on the same LAN, viewing a low-latency **cropped, cursor-following video viewport**. Cellular/internet mode is deferred until LAN safety/reliability is proven. Architecture is `React Native + TypeScript + pnpm` (Android), `Tauri 2 + React/TypeScript` (Windows tray/consent), Rust user-session host and native video stack.

## Architecture seams and dependency graph

```text
M0 UI/protocol baseline (complete)
    ├── M1-A versioned control protocol + security/state-machine test vectors
    ├── M1-B Android React Native shell and input adapter abstractions
    └── M1-C Windows Tauri/tray + Rust host shell and consent interfaces
         │
         └── M1-D LAN discovery/IP/QR and authenticated pairing
              └── M1-E trusted control DataChannel and native input dispatcher
                    └── M1-F physical Android/Windows LAN E2E + revocation
                          └── M2-A WGC capture + source consent
                               └── M2-B D3D11 crop + H.264 WebRTC
                                    └── M2-C DPI-aware cursor follow/zoom + E2E
                                         └── M3 feature parity/UX/accessibility
                                              └── M4 explicit Internet mode
```

## Workstreams, boundaries and parallelism

| Workstream | Responsibility | Owner path boundary | Parallel with |
|---|---|---|---|
| ORCH | plan, task arbitration, dependency decisions, global worklog, integration/QA | `workplan/{PLAN,TASKS,WORKLOG,README}.md` | all |
| M1-A | wire safety, packet/session validation, test fixtures, sequence/replay policy | `packages/protocol/`; own worker log | M1-B, M1-C |
| M1-B | Android RN bootstrap, typed navigation/control UI shell, permissions placeholders truthfully labeled | `apps/mobile/`; own worker log | M1-A, M1-C |
| M1-C | Windows Tauri tray/consent and native build QA; no Session 0 service or elevation | `apps/windows/**`; own worker log. Rust host separately owned by M1-D-LIVE | M1-A, M1-B |
| M1-D | Ed25519 host/phone pairing identity, source-bound challenge, local enrollment and revocation; crypto substrate published | `crates/host/**`; original M1-D worker log | M1-D-WIRE |
| M1-D-WIRE | Strict versioned TS↔Rust hello/challenge/proof JSON and transcript vectors; source published | `packages/protocol/**`; own worker log | M1-D |
| M1-D-LIVE | Native private-LAN listener, mDNS/IP/QR bootstrap, host-observed endpoint and trusted local consent plumbing; never public listener | `crates/host/**` after M1-D closure; own worker log | M1-E source foundation |
| M1-E | Independently testable Windows native input executor and DataChannel boundary; actual dispatch remains behind trusted HostSession/consent | NEW `crates/control/**` and own worker log; later host integration coordinated | M1-D-LIVE |
| M1-D-LIVE2 | Next host-only security integration: explicit local approval/session binding/revocation hooks and private LAN lifecycle, fail-closed until trusted transport and native consent are real | `crates/host/**` and NEW `workplan/workers/M1-D-LIVE2.md` only; separate from M1-E | M1-E recovery (separate ownership) |
| M1-F | Real Android↔Win11 physical A01–A17, native packaging, reconnect/revoke/performance evidence | integration/QA ownership assigned after M1-B/C/D-LIVE/E | integration only |
| M2-B | Independent D3D11 GPU crop/scale + H.264 Media Foundation media source foundation; actual WGC/WebRTC integration gated | NEW `crates/media/**` and own worker log | M1-D-LIVE and M1-E, no overlapping source |
| M2-C | Pure, deterministic cursor-follow viewport geometry, zoom/follow/stream toggle semantics, finite/negative-origin/DPI-aware crop transforms; native video mapping separately gated | NEW `crates/viewport/**` and own worker log | M2-B published source; independent of Android/Windows dirty paths |

## R6 integration wave — after source-only M1-E / M1-D-LIVE2 / M1-B-NATIVE publication

- **R6-WIN:** exclusive `apps/windows/**` and `workplan/workers/R6-WIN.md`. Improve Tauri user-session tray/consent presentation and truthful IPC without constructing local approval or DTLS identity from renderer/network data. Must remain fail-closed until native OS-local callback provenance is established.
- **R6-ANDROID:** exclusive `apps/mobile/**` and `workplan/workers/R6-ANDROID.md`. Validate SDK37/Gradle wrapper build prerequisites and advance typed private-LAN session transport, with input disabled until cryptographic channel binding and host approval are real.
- **R6-HOST (source published):** exclusive `crates/host/**` and `workplan/workers/R6-HOST.md`. Published ingress replay/size/rate/action checks and exact session teardown as `a24e5488bf31`, receipt `05cef9681096`; Cargo 48 unit + 1 doctest, fmt/check/clippy PASS. No real DTLS/OS-local callback, no input activation.
- Parent exclusively owns root package/lockfile/.gitignore, shared workplan files and Git integration. The pre-existing uncommitted M2-B worker log and untracked M2-C worker log are **worker-owned preservation**, not parent staging targets.
- Integration order: trusted Windows OS-local approval + authenticated DTLS channel → host session authorization/revoke → Android transport and input adapter → native build artifacts → A01–A17 physical E2E → real WGC/D3D11/H264/WebRTC → M2 performance → M3/M4. Independent source work may proceed in parallel, but never mark physical gates accepted from unit tests.
- Known native blockers: Android SDK37 and Gradle wrapper BAT/JAR absent, no debug APK; Windows installer not verified; media and viewport are fail-closed foundations without video output. Do not broaden network exposure or enable remote input to circumvent these gates.

## R7 authenticated-channel integration continuation (2026-10-30)

- **R7 parent:** verified `main` at `05cef9681096`, tracking `origin/main` 0 ahead/0 behind before R7 mutations. Reconciles R6 receipts, approves ownership, works only in shared `workplan/{PLAN,TASKS,WORKLOG,README}.md`; preserves dirty root `.gitignore`, `package.json`, `pnpm-lock.yaml` and peer-owned M2-B/M2-C worker logs.
- **R6-WIN:** source+QA commit `101816e396e1` published, frontend tests 4/4, typecheck, ESLint, Vite frontend build, native Cargo fmt/check/test 2/2/clippy PASS. Its own worker log follow-up was still dirty when R7 began; the live worker owns closure, not parent.
- **R6-ANDROID:** live worker validated `apps/mobile` typecheck/lint and 96/96 Vitest after repairing a test fixture; awaiting its own scoped source/worker-log commit and push at R7 start. SDK 37, build-tools 37, NDK 28.2, official Gradle wrapper BAT/JAR are missing; no APK or physical test.
- **R7-HOST-CHANNEL:** new independent specialized integrated ChatGPT deployment `3316d617-a1fe-4f5e-92a1-40edac937ae8`, child `deployment-child-678ea0d3fff84aff6093f5189a532148`, reserved exclusively `crates/host/**` and new `workplan/workers/R7-HOST-CHANNEL.md` after prior R6-HOST completed. Submission receipt at 23:51:52Z; **child authenticated connection pending at this checkpoint**, not yet implementation evidence. Validate claims/connection before tracking as active.
- **Gate order:** trustworthy, OS-observed interactive desktop and host-owned consent callback; independently authenticated DTLS/WebRTC channel with cryptographic phone enrollment binding; validated native input dispatcher cleanup; host–Android integration; platform builds/installers; physical A01–A17; then video/viewport M2–M4. Never mint authority from renderer IPC, QR hints, unprotected TCP or tests. Maintain `DisabledMissingNativeAuthority` until prerequisites really exist.

Avoid concurrent edits to root `package.json`, `pnpm-lock.yaml`, `tsconfig.base.json`, CI or shared contracts. Only parent integrates shared root config/lockfile after child proposals. Workers may commit path-scoped changes; Git FIFO serializes publication, not code design.

## R8 priority and ownership (2026-10-30 21:20 CLST)

- **Observed source checkpoint:** `main` HEAD `f698f3420ee2`, tracking `origin/main` 0 ahead/0 behind. Six pre-existing dirty/untracked files remain deliberately untouched: `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/M2-B.md`, `workplan/workers/R6-WIN.md`, and `workplan/workers/M2-C.md`. No current Zoomouse file claims; R6-WIN/ANDROID logical lifecycles are stale, not proven terminal. Do not overwrite or reassign their source paths merely from stale presence.
- **R8-PROTOCOL (P0, independent):** `packages/protocol/**` plus NEW `workplan/workers/R8-PROTOCOL.md` only. Read Rust `crates/host/src/channel_binding.rs` and Android `lan-attempt.ts` without mutation. Produce grounded cross-language transcript/defensive vectors where possible, never JS/native authorization or invented DTLS provenance. Integrated ChatGPT @MK059 deployment `2811fce0-d6ab-48a0-80d9-98893168520b`, child `deployment-child-c852e955eda7c24a2e70035c74eae1ed`, submitted 2026-10-31 00:21:12Z; authenticated child connection **not yet verified** at this checkpoint.
- **R8-WIN / R8-HOST / R8-ANDROID next (P0):** do not overlap R6-WIN/ANDROID stale recoverable ownership; first reconcile their completion/claims. The trust-critical path is a real Windows interactive-user-session provenance callback and host-owned consent, then native DTLS/WebRTC producer bound to enrolled phone and Android counterpart, host input dispatcher release guarantees, native build/install artifacts, physical A01–A17. Keep `DisabledMissingNativeAuthority` and all remote input/video OFF until all gates are met. Assign each future worker exclusive `apps/windows/**`, `crates/host/**`, or `apps/mobile/**` only after verified release/coordination.
- **M2/M3/M4:** source-only media/viewport foundations are not acceptance; preserve M2-B/M2-C uncommitted logs for a dedicated nonoverlapping recovery. M2 real WGC→D3D11→H.264→WebRTC and physical metrics follow M1, then UX parity and explicitly opted-in Internet security. Parent exclusively owns shared `workplan/{PLAN,TASKS,WORKLOG,README}.md` and root configuration; no blind lockfile staging.

## Release gates / exit criteria

**M0 — DONE (prior commit):** simulated preview + protocol, 18 local tests/lint/typecheck/build. No native app, LAN control or video.

**M1 — ACTIVE:** reproducible installable Android debug APK and Windows user-session host; host-specific approval with authenticated device identity and encrypted transport; discovery and IP fallback on private LAN; button/move/drag/scroll/key text and keyboard-up all verified; no input before approval; disconnection and revocation release held input. E2E observed on real Android + Win11, no guessed availability. Reconnect bounded, no session replay.

**M2 — BLOCKED on M1:** Windows.Graphics.Capture, GPU crop and HW H.264 with controlled fallback, WebRTC Android decoder, cursor-local coordinate translation incl. negative origins and mixed DPI, independent streaming/zoom/follow toggles, frame-time smoothing, clamping, visible capture signal, latency/power/bandwidth benchmarks.

**M3 — BACKLOG:** complete reference screenshot feature inventory (media/app/browser hotkeys, left-handed, gyro, natural scroll, keyboard modifiers, settings/history, accessibility/theme/i18n); all controls exercised E2E; no dead/mock affordances in production.

**M4 — BACKLOG:** separate threat model and explicit opt-in remote access; secure signaling/TLS, ICE/STUN/TURN, CGNAT/carrier mobile-network E2E, quotas/costs/revocation, no raw public listener.

## Security gates (non-negotiable)

No remote input or frame access without device identity, current host approval and per-session authorization; limit packet age/size/rate and monotonic sequence; protect against replay and stale keys; coalesce pointer movements; bounded queues; release held keys/buttons on reconnect/disconnect/revoke; no UAC/secure desktop; firewall LAN-scoped, no arbitrary shell/file/process execution, clipboard and recordings OFF by default. Native capture only in interactive user session, never Windows Session 0.

## Engineering evidence

Every claim references command, tested revision and environment. M1 physical device and M2 performance are presently unverified. The current `pnpm check` gate does **not** build native Android/Windows or test transport.
