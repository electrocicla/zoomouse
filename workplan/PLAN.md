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

## R9 — first native artifact build readiness vs functional acceptance (2026-10-31)

**Audited immutable reference:** `main` HEAD `6daa2afa10a67a5e2286f214d7a5e44c8fac25c4`, `origin/main` ahead=0/behind=0 at R9 startup (2026-10-31 20:25Z). Existing source receipts: protocol `44fb3ed10d32`, host `f0ea8d29dbdf` / `6daa2afa10a6`, media/viewport docs `2d5e7449baa6`. These are published source/QA, **not** native production admission. Four prior dirty files `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md` must be preserved and never bulk-staged.

**Two independent deliverables:** (i) **buildable local-development native artifacts** — unsigned Windows 11 interactive-user `zoomouse-desktop.exe` and Android **debug** APK on a reproducible owner-installed toolchain; (ii) **functional/security acceptance** — authenticated LAN pairing, host-owned interactive consent, transport-bound grants and release-safe native input, physical A01–A17; video/zoom remains a later M2 E2E gate. Compiling the first does not satisfy the second.

| Wave | Dependency and ownership | Verified exit criterion / unresolved gate |
|---|---|---|
| **R9-A Android build readiness** | `apps/mobile/**` + new `workplan/workers/R9-A-ANDROID.md` (independent child). Root config and lockfile are parent-owned. | Official Gradle 9.4.1 wrapper scripts/JAR verified from trusted distribution, Node >=22.13/pnpm10/JDK17+ compatible with RN/AGP, Android SDK platform37/build-tools37/NDK28.2.13676358 installed; strict TS/lint/unit tests; owner-authorized `:app:assembleDebug` must actually exit 0 before APK existence is claimed. No manifest INTERNET or cleartext bypass before audited secure LAN transport. |
| **R9-B Windows EXE readiness** | `apps/windows/**` + new `workplan/workers/R9-B-WINDOWS.md` (independent child); Rust host and root manifest are not worker-owned. | Windows 11 user-session build prerequisites (Rust MSVC target + VS C++ Build Tools/Windows SDK + WebView2 + pnpm), source checks; owner-authorized `tauri build --no-bundle` eventually yields unsigned `zoomouse-desktop.exe` only if build passes. `bundle.active=false` means no installer; MSI/NSIS per-user installer separately designed and approved, unsigned local-development EXE requires no Store-signing gate. |
| **R9-C native host trust** | R9-C child exclusively audits/tests `crates/host/**` + new `workers/R9-C-HOST-QA.md`; future production `crates/control/**`/`apps/windows/src-tauri/**` native integration requires separately coordinated ownership after R9-B. | Native Win32 OS-observed interactive desktop session, host-side trusted local click consent callback bound to single pending CSPRNG nonce/deadline and peer/channel, private-interface-only LAN listener, genuine DTLS exporter/cert and host identity binding, exact grant/revoke with guaranteed release. No renderer/network-minted approval; deny-all until real adapters. |
| **R9-D Android secure counterpart** | Future `apps/mobile/**` only after R9-A finishes; contract with protocol and host C. | Native WebRTC/DTLS secure transport and independently verified enrollment/channel binding, local-network permissions with correct Android target semantics, strict JNI/RN boundaries, bounded replay/session lifecycle, native mouse/keyboard mapping with held-state cleanup. No plaintext TCP/fetch workaround or implicit grant. |
| **R9-E media and viewport integration** | Future `crates/media/**`, `crates/viewport/**`, host + Android media adapters after C/D verified. | Actual user-approved WGC capture → D3D11 crop → hardware/fallback H.264 → encrypted WebRTC Android video, cursor/DPI/negative-origin follow transform, independent zoom/follow/stream toggles; physical latency, drops, CPU/GPU and battery metrics. Existing math/source tests do not count as real streaming. |

**Cascade analysis:** Root lockfile/wiring → RN Gradle plugin/Android wrapper and Tauri/Vite dependencies → native toolchain compile → host OS consent/network crypto and phone peer → authorized input lifecycle → media capture and viewport → physical acceptance. Parallel source work is permitted only with disjoint claims and pinned contract boundaries. No `any`, privileged service, Session 0, elevation, public listener, unsolicited installer/signing or device permission mutation.

**Artifact-only owner handoff (NOT executed by R9):**
- After safely reconciling the four pre-existing dirty root files, install reproducibly from the repository root with `corepack pnpm install --frozen-lockfile`; the current root `pnpm check` checks **only preview + protocol**, not mobile, Windows, Rust, or native code.
- Windows (user-owned compile): from root `pnpm --filter @zoomouse/windows typecheck`, `pnpm --filter @zoomouse/windows test`, `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml`, and separately `pnpm --filter @zoomouse/windows exec tauri build --no-bundle`. Expected *only after successful build*: `apps/windows/src-tauri/target/release/zoomouse-desktop.exe` (Cargo `[[bin]] name`). Tauri bundle setting currently disables MSI/NSIS. A locally run EXE does not implement an accepted LAN server.
- Android: first provision the **official** Gradle 9.4.1 wrapper `gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar` using trusted Gradle tooling and verify provenance (properties alone are not a wrapper). Install SDK37/build-tools37/NDK28.2.13676358/JDK compatible with AGP9; validate mobile TS/lint/unit tests. Only with owner's explicit build authorization, run `cd apps/mobile/android` then `.\\gradlew.bat :app:assembleDebug` from Windows PowerShell. Expected *only on success*: `apps/mobile/android/app/build/outputs/apk/debug/app-debug.apk`. No APK presently verified.
- Gating: this delegation **does not authorize running** Windows package/build/sign/install, Android `assembleDebug`, native consent/permissions, or A01–A17 physical tests. No Store signature is required to test a locally built unsigned development EXE; installer signing/distribution policy is a separate decision.
