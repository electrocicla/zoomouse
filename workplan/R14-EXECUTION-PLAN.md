# Zoomouse R14 — LAN-first native integration, provenance and build-readiness plan
Date: 2026-11-06 Chile (runtime UTC 2026-11-07). Orchestrator task `deployment-child-ab23ffa5e84abcbdded80198b5bbb3c1`, own session/lifecycle `92c77675-a98a-4015-a94a-cbceaea068d0`; authorized Zoomouse rootIndex13, canonical `github.com/electrocicla/zoomouse`, `main`.

## P0 immutable startup boundary
- Observed `main` at `1fb3b9ea6c83c211d3af11115840adede6f85d26`, tracking `origin/main`, ahead0/behind0; 15 unstaged/untracked paths, no staged files.
- Four **inherited/protected and excluded from all R14 commits**: `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md`. Do not stash, reset, overwrite, or bulk-stage; root dependency/lockfile provenance requires an owner decision.
- Eleven **unpublished R13 Android** paths: `apps/mobile/App.tsx`, `apps/mobile/android/app/build.gradle`, `apps/mobile/android/app/src/main/java/com/zoomouse/mobile/SecureLanModule.kt`, `apps/mobile/src/domain/android-native-lan.test.ts`, `apps/mobile/src/domain/android-native-lan.ts`, `apps/mobile/src/domain/lan-attempt.test.ts`, `apps/mobile/src/domain/lan-attempt.ts`, `apps/mobile/src/screens/OnboardingScreens.tsx`, `apps/mobile/android/app/src/main/java/com/zoomouse/mobile/SecureLanAttemptFence.kt`, `apps/mobile/android/app/src/test/java/com/zoomouse/mobile/SecureLanAttemptFenceTest.kt`, `workplan/workers/R13-ANDROID.md`. They are not accepted, QA'd, committed or published by R13. Treat as preserved independent source until bounded recovery proves otherwise.
- Prior R13 source receipts: media `d851bb4ca5ac` (12 tests), host source `83bd4136eaf9` and log `5f2bedb70c8e` (64 Rust unit +1 doctest), Windows source `2aa23c32a14e` and log `1fb3b9ea6c83` (Rust/TS checks). These are **not** live native/physical validation.

## Cascading source architecture and gates
```
Root pnpm+RN0.88/Gradle9.4.1 wrapper+SDK37/JDK/NDK  [artifact preflight]
   └─ RN strict TS AppState & LanAttemptCoordinator → native bridge → Kotlin SecureLanAttemptFence
             └─ host pairing/enrollment key + protocol transcript (untrusted discovery/IP/QR only)
                └─ REAL native WebRTC DTLS/SCTP handshake with host+phone independently
                   observed cert fingerprints/exporter/stream and enrolled identity
                   └─ crates/host channel_binding + consent/ingress replay/rate/generation
                     + Windows interactive desktop OS-observed *host-local* approval
                       └─ host-owned time-scoped grant → crates/control native input
                          + revoke/key-up/lock/background/peer-switch/teardown tests
                            └─ safe owner-built EXE/APK, physical A01–A17
                               └─ WGC captured GPU surface → D3D11 crop → MF H264
                                  → encrypted WebRTC sender → Android decoder
                                  → DPI/negative-origin cursor-follow viewport → M2 metrics
```
Current `crates/host/src/lib.rs` deliberately has no production DTLS adapter, `channel_binding.rs` is proof validation with **no native producer**, and `ingress.rs` is crate-private. `apps/windows/src-tauri/src/lib.rs` deliberately has no native approval constructor/listener and a disabled input backend; `crates/control/src/native.rs` has a private Windows SendInput sink with interactive-desktop guard, NOT a host grant. Android Kotlin module is deny-only with no socket; even resolved JS bridge results must not authorize remote input. `crates/media/src/native.rs` validates WGC descriptors but has no GPU encoder/WebRTC; `crates/viewport` is geometry-only. Native trust and media E2E remain BLOCKED.

## Task list and exclusive ownership
| ID | Priority | Owner paths | Completion evidence and gate |
|---|---|---|---|
| R14-01 parent | P0 | `workplan/R14-EXECUTION-PLAN.md`, `workplan/TASKS.md`, `workplan/WORKLOG.md` only | Initial plan before source changes; timestamped downstream source receipts, status/decision reconciliation; scoped Git publication. |
| R14-02 Android recovery | **P0 first deployment** | `apps/mobile/**`, NEW `workplan/workers/R14-ANDROID-RECOVERY.md`; review existing `R13-ANDROID.md`, only include it in exact commit if authorship/provenance preserved | Recover existing 11 unpublished paths without loss; fail-closed attempt nonce/replay/cancellation/foreground+bridge tests. Host-tracked mobile typecheck/lint/test/android:preflight with actual run IDs/exits. Kotlin JUnit only if real Gradle toolchain. Exact own-path stage/commit/push origin0/0. No APK and no root dirty files. |
| R14-03 host native provenance integration | P0 after separate preflight | `crates/host/**` and NEW `workplan/workers/R14-HOST-TRUST.md` | Inspect real native adapter availability and backend-observed DTLS exporter/cert/SCTP; implement minimal fail-closed real integration/seam and adversarial tests where feasible. Never fabricate verified producer/grant or enable plaintext. Rust fmt/check/test/clippy, scoped commit+push. |
| R14-04 Windows native OS-local consent | P0 after separate preflight | `apps/windows/**`, `crates/control/**`, NEW `workplan/workers/R14-WINDOWS-TRUST.md` | Independently verify Win11 user-session/desktop/host-local consent, revoke and held-input cleanup; minimal source improvement with strict negative tests. No renderer-minted approval or remote input enablement without real DTLS producer + approval. Rust/TS QA; scoped commit+push. |
| R14-05 protocol boundary | P0 conditional | `packages/protocol/**`, NEW `workplan/workers/R14-PROTOCOL.md` | Only if an actual cross-language incompatible wire contract is established by host+Android workers; otherwise defer; no speculative wire change. |
| R14-06 M1 physical gates | OWNER | Builds/installs/approval and `workplan/ACCEPTANCE-M1.md` | A01–A17 all NOT RUN. Actual signed transport, native local grant, real Android/Win11 hardware, disconnection/lock/revoke timing must be measured, never inferred. |
| R14-07 M2 media | P1 deferred until M1 | `crates/media/**`, `crates/viewport/**`, Android/Windows media consumers in separately serialized scope | Real WGC/D3D11/MF H264→WebRTC sender→Android decode and cursor following; driver/GPU and p95 numbers currently unknown. |
| R14-WAN | DEFERRED | separate owner-approved work | Do not enable M4/public listen/relay. |

## Coordination and publication discipline
- First authenticated R14 Android recovery child, then independent host/Windows workers **only after** prior child `connectedAt` and new deployment preflight; exclusive path claims. No child orchestrators/successors. Parent alone updates shared TASKS/WORKLOG; worker notes live in independent `workplan/workers/R14-*.md`.
- Use current native Agents Global policy, own host-injected IDs/epoch, workspace root13, snapshot/inbox before collisions. Git FIFO: stage only exact actor-owned paths, commit/push, verify SHA and origin/main 0/0; no force/reset/all-stage. Source tests with actual exit codes never imply device acceptance. Report missing toolchain as blocked rather than generate wrapper/SDK or change permission defaults.
- Owner commands **not executed** here: Windows `pnpm --filter @zoomouse/windows exec tauri build --no-bundle` (raw unsigned EXE, `bundle.active=false`, no installer); Android PowerShell inside `apps/mobile/android`: `.\gradlew.bat :app:assembleDebug` only after authenticated official Gradle 9.4.1 BAT/JAR, SDK Platform37, BuildTools37, NDK28.2.13676358 and compatible JDK; owner separately authorizes packaging and physical installation. Root `pnpm check` validates preview/protocol only.
