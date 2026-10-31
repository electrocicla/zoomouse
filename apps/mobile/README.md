# Zoomouse Android — M1-B scaffold

React Native **0.88.1 / React 19.3.0**, TypeScript strict (no `any`), Android New Architecture/Hermes, pnpm workspace package `@zoomouse/mobile`. This is a **native Android source scaffold**, not a delivered APK or tested Android build. It is not yet connected to any Windows host.

## Current screens
- LAN devices: honest empty discovery state (no demo computer list).
- Manual IPv4: local RFC1918 validation; no connection attempt.
- QR: informational screen; camera permission is **not requested**.
- Windows approval: describes mandatory host consent, does not impersonate approval.
- Touchpad, clicks, keyboard and zoom: disabled or explicitly marked for M1-D/E/M2. No input is sent.

`src/domain/transport.ts` declares LAN interfaces for discovery, host approval and authorized session. `createUnavailableLanClient` rejects all operations; `src/domain/session.ts` keeps the default offline and **never unlocks remote input** in this shell; a future native adapter must provide independently verified Windows consent, transport authentication and session binding before any real input integration. A JavaScript phase value cannot grant input authority.

## Android source / security boundary
`android/app/src/main/AndroidManifest.xml` requests **no network, camera, storage or nearby-device permissions** in the release manifest. `android/app/src/debug/AndroidManifest.xml` adds INTERNET and cleartext **for Metro development only**. No public listener, cellular relay or background service is implemented.

The Android source tree includes Gradle build/settings, Kotlin entry points, release/debug manifests, New Architecture flags and RN Metro/Babel wiring. **Gradle wrapper executable/JAR provisioning and full native build remain unverified**; do not claim this is independently buildable until the wrapper and Android toolchain have been provisioned and a build passes. The root pnpm lockfile remains parent-owned.

## Next integration / validation
1. Parent integrates RN/pnpm dependencies into `pnpm-lock.yaml` and validates version compatibility. On a JDK17/Android SDK37/Node22.13+ setup, install dependencies and provision Gradle wrapper from a trusted official template or Gradle installation (including `gradle-wrapper.jar`); do not invent a binary.
2. Run `pnpm --filter @zoomouse/mobile typecheck`, `pnpm --filter @zoomouse/mobile test`, and Android `assembleDebug` after toolchain availability. Test install/launch on physical Android; no APK acceptance without real build.
3. M1-D provides authenticated discovery/IP/QR and explicit Windows approval. M1-E provides trusted input channel with grant freshness and release-held-input cleanup. M2 adds native media/zoom. M4 internet follows separate security approval.

**Recovery audit (2026-10-30):** Gradle properties now include the RN 0.88 / AGP 9 compatibility opt-outs (`android.builtInKotlin=false`, `android.newDsl=false`); mobile manifest declares direct React Native Gradle and community CLI plugin dependencies so strict pnpm workspace linking can resolve them. Root lockfile and toolchain are orchestrator-owned. The JS input eligibility helper is deliberately deny-all until the native mutually authenticated session implementation is provided. Changing only a UI `phase` value cannot authorize mouse/keyboard.

**Permission follow-up:** Main release manifest currently omits `INTERNET` because no production LAN transport exists. M1-D must add the normal `INTERNET` manifest permission when a native LAN implementation is integrated, without enabling cleartext or WAN mode. With Android target SDK 36, Android 17's broad `ACCESS_LOCAL_NETWORK` runtime permission is not yet required; on eventual target SDK 37 migration, adopt the official local-network permission flow (or supported privacy-preserving system picker) before attempting discovery/network sockets. Never use manifest permissions as the sole outbound network restriction.

See `workplan/workers/M1-B.md` for observed receipts and blockers.

## R6-ANDROID: strict native LAN transport seam (source only)

`src/domain/lan-attempt.ts` adds a single-flight `LanAttemptCoordinator`. It validates manual-IP, mDNS and QR-key-pin intents before calling a future audited Android `NativeSecureLanPort`; every open/close operation has a bounded attempt ID and cancellation signal. Disconnect invalidates pending callbacks before async native cleanup; cleanup failure permanently denies reuse. Even a native open callback is marked `native-link-unverified` (observation only) and `canSendInputFromLanAttempt` always returns false. This **cannot** construct a `HostApprovedSession` or authorize input.

`createUnavailableNativeLanPort` opens no socket. No native WebRTC/DTLS dependency, channel exporter/pinned Ed25519 host key, authenticated pairing signer, Windows OS-local approval receipt, session grant/replay verifier, Android INTERNET permission, or release-held-input native hooks exist. Do **not** bridge with RN fetch or plaintext TCP. Android SDK platform37, build-tools37, NDK28.2.13676358, official Gradle wrapper BAT/JAR and an actual debug APK remain independent prerequisites; the Gradle wrapper properties URL is not an installed wrapper. See `workplan/workers/R6-ANDROID.md` for current receipts. A01–A17 physical acceptance remains NOT RUN.

## R9: reproducible Android debug-APK handoff (source-only; no APK produced)

**Observed blocker:** `android/gradle/wrapper/gradle-wrapper.properties` names official Gradle **9.4.1**, but `android/gradlew`, `android/gradlew.bat`, and `android/gradle/wrapper/gradle-wrapper.jar` are **absent** from the repository. The configuration requests `compileSdk=37`, build-tools `37.0.0`, NDK `28.2.13676358`, `minSdk=24`, and `targetSdk=36`. An earlier actual host probe (R6) found SDK platform 35/36 only, JDK present, and missing required 37/build-tools/NDK/wrapper; no fresh on-device readiness claim is made here. React Native 0.88.1 requires the installed workspace dependencies and the Gradle plugin.

**Security pin:** `distributionSha256Sum=2ab2958f2a1e51120c326cad6f385153bb11ee93b3c216c5fccebfdfbb7ec6cb` is Gradle's published **9.4.1 bin ZIP** digest; `gradle-wrapper.jar` must hash to `55243ef57851f12b070ad14f7f5bb8302daceeebc5bce5ece5fa6edb23e1145c`. Official verification: https://gradle.org/release-checksums/ and https://docs.gradle.org/current/userguide/gradle_wrapper.html . Do not hand-write a wrapper JAR, borrow an unverified artifact, or disable checksum validation.

### User-owned Windows prerequisites (do not run packaging from this worker)

1. Install Node.js >=22.13, pnpm for this repo, **JDK 21** (or supported >=17) and set `JAVA_HOME` to its real JDK directory. Check `java -version` and `javac -version`; `JAVA_HOME/release` must declare its Java version. Set `ANDROID_HOME` to the installed Android SDK path. If `ANDROID_SDK_ROOT` is also set, make both resolve to the same SDK.
2. Via **Android Studio SDK Manager**, install Android **SDK Platform 37**, **Build-Tools 37.0.0**, **NDK 28.2.13676358**. Official SDK guidance: https://developer.android.com/about/versions/17/setup-sdk . Keep the app's target SDK at 36 until a separate permission/security review authorizes change.
3. After the parent resolves the **foreign DIRTY** root `package.json`/`pnpm-lock.yaml` and confirms a coherent lockfile, from repo root run `pnpm install --frozen-lockfile`. Do not edit the root lockfile from this Android workstream.
4. Obtain the official **Gradle 9.4.1 -bin ZIP** from `https://services.gradle.org/distributions/gradle-9.4.1-bin.zip`; before extracting, verify its SHA-256 equals the pinned bin digest above (Windows PowerShell: `(Get-FileHash <downloaded-zip-path> -Algorithm SHA256).Hash.ToLowerInvariant()`). Extract the verified archive to a temporary folder. From `apps/mobile/android`, run the verified `gradle-9.4.1/bin/gradle.bat wrapper --gradle-version 9.4.1 --distribution-type bin --gradle-distribution-sha256-sum 2ab2958f2a1e51120c326cad6f385153bb11ee93b3c216c5fccebfdfbb7ec6cb`. Use the actual extracted path to `gradle.bat`. Verify the generated JAR's SHA-256 with `Get-FileHash` against the official wrapper digest; reject mismatches. Retain the official `gradlew`, `gradlew.bat`, `gradle-wrapper.jar` and existing pinned properties as version-controlled project source after review. **No binary is generated, downloaded, installed or committed by this worker.**
5. Run from repository root:
   ```powershell
   pnpm --filter @zoomouse/mobile android:preflight
   pnpm --filter @zoomouse/mobile typecheck
   pnpm --filter @zoomouse/mobile lint
   pnpm --filter @zoomouse/mobile test
   cd apps/mobile/android
   .\gradlew.bat --no-daemon :app:assembleDebug
   ```
   The read-only preflight fails closed on missing wrapper scripts, incorrect wrapper hashes, SDK/JDK/NDK, Node version and RN plugin; passing the preflight **does not prove Gradle compilation or a working APK**. Only a successful real `assembleDebug` run could generate `apps/mobile/android/app/build/outputs/apk/debug/app-debug.apk`; output existence, install/launch and physical A01–A17 acceptance require independent verification.

**Scope separation:** This debug variant includes `INTERNET` and cleartext only for the React Native Metro development server; the main/release manifest remains without `INTERNET` and has `usesCleartextTraffic=false`. The debug APK is **not** a secure LAN product and must not be presented as remote-control ready. There is no authenticated Android DTLS producer, Windows host consent receipt, native input grant or actual low-latency video. No insecure transport exception, production INTERNET permission, signing or device install is authorized by this source-only task.
