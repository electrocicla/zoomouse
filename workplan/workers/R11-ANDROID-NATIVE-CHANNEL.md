# R11 Android native-channel recovery — source QA (2026-11-04)

## Provenance and ownership

- Independent recovery task `deployment-child-6f3d71372cc653905a11c41c2066d3c3`, lifecycle `d8697ceb-8b1d-4161-80be-20f5ec267d9c`, verified authorized workspace rootIndex13 `Zoomouse`, remote `github.com/electrocicla/zoomouse`, branch `main`.
- Recovered exactly the original worker's **nine preserved, initially uncommitted** files, without rewriting passing source: `apps/mobile/App.tsx`, `android-readiness.mjs`, `android-readiness.test.mjs`, `android/app/src/main/java/com/zoomouse/mobile/MainApplication.kt`, new `SecureLanModule.kt`, new `SecureLanPackage.kt`, `src/screens/OnboardingScreens.tsx`, new `src/domain/android-native-lan.ts`, new `src/domain/android-native-lan.test.ts` (all under `apps/mobile/`).
- **Protected, pre-existing owner modifications untouched**: `.gitignore`, root `package.json`, root `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md`. Media files and shared parent workplan also outside recovery ownership.

## Actual tracked Windows PowerShell QA

All commands executed in the real `I:\Zoomouse` selected repository using the governed Not-Code terminal, run-bound completion markers, **not simulated/assumed**.

| Command | Tracked run ID | Exit | Evidence |
| --- | --- | --- | --- |
| `node --test apps/mobile/android-readiness.test.mjs` | `termrun_b29da4800044408ebb4f1408e60325bd` | **0** | 5 tests PASS. |
| `corepack pnpm --filter @zoomouse/mobile exec vitest run` | `termrun_fdb8d81e185d43d19492317b1cc3c677` | **0** | 6 test files, **104 tests PASS**, including 6 Android bridge negatives. |
| `corepack pnpm --filter @zoomouse/mobile typecheck` | `termrun_c03bef498e8a47e6a3a38bba047d9983` | **0** | TypeScript compiler passes. |
| `corepack pnpm --filter @zoomouse/mobile lint` | `termrun_8bc79fdfbf7240cc9795f154d35846b3` | **0** | ESLint passes. |
| `corepack pnpm --filter @zoomouse/mobile android:preflight` | `termrun_f16b385855704afe875ea071d2d7a38a` | **1 (expected build blocker)** | Node, Gradle distribution URL/checksum, JDK release/tools, RN Gradle plugin PASS; official wrapper scripts `gradlew`/`gradlew.bat`, official verified wrapper JAR, `ANDROID_HOME`, SDK Platform 37, Build-Tools 37.0.0, and NDK 28.2.13676358 BLOCK. |

All five runs reached tracked terminal completion and reported exact exit code. Android Gradle assembleDebug was **not attempted**, because its actual prerequisites are absent. No debug APK exists by evidence from this QA; artifact existence not verified.

## Security and integration boundary

- Kotlin `ZoomouseSecureLan` is registered in `MainApplication` but **deliberately deny-only** (`E_NATIVE_CHANNEL_UNAVAILABLE`). No socket, `INTERNET` permission, native credential, or input capability is enabled.
- TypeScript bridge treats QR pinned-key material as an *untrusted hint*, refuses manual-IP/mDNS link creation and a forged successful native resolution, and exercises late-open cancellation and close-failure latching. JS cannot authorize input; native OS-local Windows consent and a mutually authenticated DTLS/peer key producer are absent.
- UI makes unavailable transport explicit; no camera/QR scanning or real connection is claimed.
- **Status**: Android fail-closed SOURCE QA **PASS**; **NATIVE CHANNEL NOT ACCEPTED**. Physical A01–A17, real Win11-to-Android pairing/stream/input, Win11 EXE and Android APK **NOT RUN / NOT ACCEPTED**.
- To unblock a real APK build, provision an authentic verified official Gradle 9.4.1 wrapper, configure Android SDK 37/BuildTools 37.0.0/NDK 28.2.13676358, re-run preflight, then execute owner-authorized `apps/mobile/android/gradlew.bat :app:assembleDebug` and run native device acceptance. Do not substitute an unverified wrapper binary or infer trust from a build alone.

## Publication scope

Only nine `apps/mobile/` files above plus this newly created `workplan/workers/R11-ANDROID-NATIVE-CHANNEL.md` are eligible for this recovery worker's scoped commit. Publication SHA/upstream receipt must be evidenced separately, not inferred from this document.
