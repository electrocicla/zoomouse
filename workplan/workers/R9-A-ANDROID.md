# R9-A — Android debug APK source-readiness and QA receipt

## Identity / scope
- Task: `deployment-child-9dd2afe00c49a0fc9f8575e748c1a498`; instance: `web-child-9dd2afe00c49a0fc9f8575e748c1a498`; epoch: `1847921b-497e-467f-9080-60febd4386d0`.
- Parent: `deployment-child-47eb49e2a8085a1dc072ba657b1bede7` (ZOOMOUSE-R9-RELEASE-READINESS-ORCHESTRATOR).
- Authorized workspace: rootIndex 13 Zoomouse, `github.com/electrocicla/zoomouse`, branch `main`; initial `HEAD=6daa2afa10a67a5e2286f214d7a5e44c8fac25c4` and `origin/main` 0/0.
- Exclusive mutation only `apps/mobile/**` and this NEW worker log. Foreign root DIRTY `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md` and all R9-B/R9-C/parent work remain untouched.

## High-level source audit and dependency cascade
1. Existing React Native **0.88.1** / React **19.3.0** native shell uses compileSdk37, buildTools37.0.0, NDK28.2.13676358, targetSdk36, minSdk24, Gradle9.4.1 wrapper properties; new architecture/Hermes/AGP9 opt-outs already in source. JDK17+ needed, JDK21 recommended.
2. Native Android wrapper **NOT in repo**: `android/gradlew`, `android/gradlew.bat`, `android/gradle/wrapper/gradle-wrapper.jar` absent. Historical R6 host read-only toolchain probe found SDK 35/36, older NDK, no global Gradle, no APK; no new host SDK installation is asserted.
3. **Security boundary:** main AndroidManifest requests no INTERNET/camera/nearby permission and disallows cleartext; debug manifest allows INTERNET/cleartext for Metro **only**. Android LAN/native DTLS authenticator, host-owned Windows consent, input enablement and video remain NOT IMPLEMENTED. No permission/transport/authentication relaxed.
4. Added official Gradle 9.4.1 **bin distribution SHA pin** to existing wrapper properties, Node read-only preflight with strict official wrapper JAR SHA check, four deterministic fail-closed synthetic unit tests, app-local preflight/test scripts, and README Windows owner handoff.
5. Parent must reconcile dirty root lockfile before `pnpm install --frozen-lockfile`; user must obtain verified official Gradle distribution/JAR/scripts and install platform37/build-tools37.0.0/NDK28.2/JDK then run real `:app:assembleDebug` on Windows. Expected APK **only conditional on success**: `apps/mobile/android/app/build/outputs/apk/debug/app-debug.apk`. Source-only tests cannot establish on-device A01–A17, host approval, input, or media.

## Exact owned changes
- NEW `apps/mobile/android-readiness.mjs`: read-only Node preflight for official Gradle bin SHA, Gradle JAR SHA, wrapper scripts, Node22.13+, JDK, SDK37/build-tools37/NDK28.2, RN Gradle plugin, mismatched SDK roots.
- NEW `apps/mobile/android-readiness.test.mjs`: Node built-in unit tests for absent prerequisites, unverified synthetic JAR, insecure Gradle URL/invalid digest/conflicting SDK roots, old Node/JDK.
- UPDATE `apps/mobile/android/gradle/wrapper/gradle-wrapper.properties`: `distributionSha256Sum=2ab2958f2a1e51120c326cad6f385153bb11ee93b3c216c5fccebfdfbb7ec6cb`.
- UPDATE `apps/mobile/package.json`: `android:preflight` and Node preflight tests before existing Vitest suite.
- UPDATE `apps/mobile/README.md`: secure official wrapper provisioning, exact owner build/QA commands and output, clear release/debug network boundary.
- NEW `workplan/workers/R9-A-ANDROID.md`: this report.

Official SHA evidence: https://gradle.org/release-checksums/ ; wrapper provenance: https://docs.gradle.org/current/userguide/gradle_wrapper.html ; platform37 setup: https://developer.android.com/about/versions/17/setup-sdk .

## Validation / coordination (host-tracked receipts, 2026-10-31)
- Root `pnpm test:run`: validation task **`validation_c900af2d6aea4dea`**, run **`run_293d27a5a8cd4330`**, exit **0** at 20:36:41Z; the root script **excludes mobile**, so it is supporting regression evidence only.
- Mobile `pnpm --filter @zoomouse/mobile test`: terminal `agent-term-1793479030432-19`, run **`termrun_9b192f7f153e4c0080905d5fbf5090e0`**, exit **0** at 20:48:50Z: **4/4 Node built-in preflight negative tests plus 96/96 existing Vitest tests across 5 files PASS**.
- Mobile `pnpm --filter @zoomouse/mobile typecheck`: run **`termrun_7f0de43385334e269298f727b9408116`**, exit **0** at 20:49:05Z, strict TypeScript PASS.
- Mobile `pnpm --filter @zoomouse/mobile lint`: run **`termrun_557bc01cab184b188a2177497d91bb65`**, exit **0** at 20:50:01Z, ESLint PASS.
- Mobile `pnpm --filter @zoomouse/mobile android:preflight`: run **`termrun_896b73394b984d18ac6ec159acd053b6`**, exit **1 AS EXPECTED** at 20:50:17Z, fail-closed, not native build readiness. It reports PASS for actual JDK tools/release and installed RN Gradle plugin, BLOCK for SDK root/platform37/build-tools37/NDK28.2 and missing official wrapper. This is **negative readiness evidence**; it must not be treated as a failing unit test or a built APK.
- First two scoped terminal launch attempts were correctly rejected **before execution** by competing R9-B reservations. Parent shared Git turn and R9-C host claims were later released; no duplicated accepted command. Critical notifications acknowledged. Source-only checks above all have authoritative tracked exits.
- Agents Global: own lifecycle authenticated root13, parent READY sent, R9-B and R9-C collisions acknowledged, all own source file claims released. Parent docs commit `3be7aa988d85`, R9-C source commit `9d05e27dba14`, each published separately by rightful owners. No unrelated Git paths staged.
- **NOT RUN**: Android `assembleDebug`, Gradle wrapper generation/download, signing, installation, physical A01–A17, release builds, network sockets, remote input or video.
- Git source commit and push are performed as a separate explicit six-file scope; verify actual hash and origin/main ahead=0 behind=0 before closing this task. No historical parent/index paths are included.
