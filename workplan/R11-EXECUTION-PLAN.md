# Zoomouse R11 — native trust closure, build artifacts and acceptance (2026-11-04)

## Audited baseline

- Authorized MK059 rootIndex13 **Zoomouse**, canonical `github.com/electrocicla/zoomouse`, branch `main`, verified HEAD `2249c579c7b8cc80653882ed8af817b69094e760`, upstream ahead0/behind0 on 2026-11-04 17:46 UTC.
- Four **pre-existing modified files**: `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md`. Preserve byte-for-byte; never reset, stage, overwrite or imply ownership. Root config/lockfile reconciliation requires a provenance/ownership decision.
- R9 Android readiness source: `0b7216c7e2ef`; Windows readiness source: `7d6eeb29d8b6`; host R9 security: `9d05e27dba14`. R10 cancelled late Android LAN-open cleanup: `9f2a28da0d53`, test evidence in `workplan/workers/R10-ORCH-MOBILE-RACE.md`. These are **SOURCE checks only**, not Android/Windows physical acceptance.
- No active Zoomouse child agents at R11 startup. Parent task `zoomouse-r11-orchestration-20261104` connected with its own explicit lifecycle; deployment transport preflight ready/shared. No authenticated native DTLS/WebRTC exporter or OS-local approval producer, real input/stream, physical A01–A17, APK or EXE artifact receipt known.

## Workstream ownership, in dependency order

| Phase | Exclusive owner paths | Work and acceptance boundary |
|---|---|---|
| R11-WIN-SESSION (P0) | `apps/windows/**` + NEW `workplan/workers/R11-WIN-NATIVE-SESSION.md` | Implement smallest real Windows OS-observed interactive session/desktop signal and fail-closed transitions if native APIs/toolchain allow. Do not mint local approval from renderer/IPC or enable input. Run Windows TS and Tauri Rust compile/test/clippy where possible; record readiness for raw EXE separately. |
| R11-ANDROID-CHANNEL (P0) | `apps/mobile/**` + NEW `workplan/workers/R11-ANDROID-NATIVE-CHANNEL.md` | Implement native secure LAN client only with actual peer identity/DTLS proof. Otherwise produce useful typed fail-closed native integration and negative tests, protect delayed-open cancellation and revocation. Validate SDK37/Gradle9.4.1 official wrapper/NDK availability and record exact APK build blocker. No plaintext fallback or INTERNET enablement without security review. |
| R11-HOST-CHANNEL (P0) | `crates/host/**` + NEW `workplan/workers/R11-HOST-CHANNEL.md` | Actual host-side independently evidenced channel trust and OS-local consent wiring where feasible. No simulated DTLS/native proof constructors or release of held keys bypass. Add adversarial QA for session switching, replay, revoke, cleanup and trust edge. Cargo fmt/check/test/clippy. |
| R11-ORCH (P0) | `workplan/{R11-EXECUTION-PLAN,PLAN,TASKS,WORKLOG,README}.md` only | Parent reconciles workers, Git receipts, strict ownership, validation, native vs source acceptance, and expected .exe/.apk build commands; preserve four foreign dirty paths. |
| R11-NATIVE-E2E (P0 after trust) | separate explicit assignment, no overlapping edits | Reproducible owner-authorized Windows raw `.exe` and Android APK builds, signed host-owned pairing, interactive native consent, encrypted LAN input with releases, install, A01–A17 actual device evidence. **UNACCEPTED** until run. |
| M2 / M3 / M4 | later disjoint assignments after M1 | WGC capture → D3D11 crop → H.264 WebRTC → Android decoding and cursor zoom, p95 metrics; reference feature parity; opt-in Internet with TURN/CGNAT threat model. Do not claim implemented by source geometry tests. |

## Engineering and publication gates

1. Start each child deployment **sequentially** through integrated ChatGPT @MK059; require exact child Agents Global `connectedAt` before another start. No stale instance impersonation or duplicate Send. Reconcile inbox, claims, dependencies and upstream before any mutation.
2. Strict TypeScript, **no `any`**, minimal SRP/SOLID, OS-owned trust and fail-closed defaults. Never elevate, approve a host prompt from web/renderer, open a public input listener, or fake device/DTLS evidence.
3. Workers stage exactly owned files and append own worker reports; run targeted source tests with real exit codes, then commit/push with verified upstream ahead0/behind0. Parent controls shared plan/task/worklog and does not consume worker-owned dirties.
4. Artifact/build and physical device commands are separately gated by real prerequisites and any repository owner-only package approval. Never infer an actual `.exe` or `.apk` from a passing source test.
5. Keep explicit statuses `SOURCE DONE`, `NATIVE NOT ACCEPTED`, `BLOCKED`. Deny transport/input/capture by default until actual Win11/Android proof is available.

## Reproducible owner-run artifact commands — not executed in R11 plan

Windows x64 after VS C++ Build Tools, Windows SDK, WebView2, Rust MSVC, Node22, pnpm10 and legitimate root dependency install:

```powershell
corepack pnpm install --frozen-lockfile
corepack pnpm --filter @zoomouse/windows exec tauri build --no-bundle
Test-Path .\\apps\\windows\\src-tauri\\target\\release\\zoomouse-desktop.exe
```

Android after official wrapper BAT/JAR, JDK17+ supported by Gradle/AGP, Android SDK37, Build-Tools37, NDK28.2.13676358, and appropriate RN plugin/package graph:

```powershell
corepack pnpm --filter @zoomouse/mobile android:preflight
cd apps/mobile/android
.\\gradlew.bat :app:assembleDebug
Test-Path .\\app\\build\\outputs\\apk\\debug\\app-debug.apk
```

A passing build yields artifact existence only. It is **not** authentication, functionality, performance, or real-device acceptance.
