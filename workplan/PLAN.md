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
- **R6-HOST (planned):** exclusive `crates/host/**` and `workplan/workers/R6-HOST.md`. Audit/implement actual trusted encrypted transport and native authority seam where supported; never expose current plaintext TCP test adapter or mint approval receipts from remote input.
- Parent exclusively owns root package/lockfile/.gitignore, shared workplan files and Git integration. The pre-existing uncommitted M2-B worker log and untracked M2-C worker log are **worker-owned preservation**, not parent staging targets.
- Integration order: trusted Windows OS-local approval + authenticated DTLS channel → host session authorization/revoke → Android transport and input adapter → native build artifacts → A01–A17 physical E2E → real WGC/D3D11/H264/WebRTC → M2 performance → M3/M4. Independent source work may proceed in parallel, but never mark physical gates accepted from unit tests.
- Known native blockers: Android SDK37 and Gradle wrapper BAT/JAR absent, no debug APK; Windows installer not verified; media and viewport are fail-closed foundations without video output. Do not broaden network exposure or enable remote input to circumvent these gates.

Avoid concurrent edits to root `package.json`, `pnpm-lock.yaml`, `tsconfig.base.json`, CI or shared contracts. Only parent integrates shared root config/lockfile after child proposals. Workers may commit path-scoped changes; Git FIFO serializes publication, not code design.

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
