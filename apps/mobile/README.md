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
