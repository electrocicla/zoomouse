# Zoomouse R17 — real native encrypted LAN implementation (2026-11-20 CLST)

## Authorized baseline / authority
- Owner-authorized fresh R17 logical task: deployment-child-b07ba13474c507da4c1a7cd20669602c, instance web-child-b07ba13474c507da4c1a7cd20669602c, epoch f480261c-a682-4172-b711-75e4b4ed4c8e, rootIndex13 Zoomouse. Parent zoomouse-r17-owner-review-deploy-20261120-1136 is context only.
- Canonical github.com/electrocicla/zoomouse branch main HEAD=origin/main 765c165db92ea21464440b8b39ca3850eb055811, ahead0 behind0. R16 completed, source-only checks verified there. Fresh commit and tool status are separately recorded.
- PRESERVE EXCLUSIVELY FOREIGN: .gitignore, root package.json, pnpm-lock.yaml, workplan/workers/R6-WIN.md (modified) and workplan/workers/R13-ANDROID.md (untracked). Never touch/stage/reset/stash these. R16 docs immutable. No parallel Zoomouse work claims on admission.
- No blanket permission to install dependencies or SDKs, to build/sign/install, to change machine-wide network policy or to approve desktop consent. Approval for source and scoped tests only.

## Dependency/call graph observed in source
1. QR/source location: packages/protocol/src/pairing-wire.ts -> apps/mobile/src/domain/{lan-bootstrap,android-native-lan,lan-attempt}.ts validates locator, never grants authority.
2. Native phone: AndroidManifest.xml lacks INTERNET; SecureLanModule.kt, SecureLanAttemptFence.kt and SecureLanPairingProof.kt implement deny-only bridge with ephemeral, one-shot JCA Ed25519 proof and background cancellation. No Android authenticated socket, pinned remote certificate nor SDK/native compiled proof.
3. Host: crates/host/src/lan.rs validates private IPs; pairing.rs has enrolled-phone signature checks, channel_binding.rs checks signed transcript over *caller-supplied* observations; ingress.rs, admission.rs and boundary.rs require separate trusted channel/Windows OS consent. Cargo.toml has no TLS/WebRTC native provider. pairing_tcp.rs is private plaintext test fixture — NEVER promote to production listener.
4. Windows: apps/windows/src-tauri/src/lib.rs -> LocalHostState<DisabledInput>, desktop_observer.rs detects desktop only. No host-observed protected transport, interactive OS-local device-specific grant or guarded native input dispatch.
5. M1 (physical LAN A01–A17) requires native encrypted host+Android association, independent host identity pin and enrolled phone proof, fresh session-bound OS-consent, safe input/cleanup. M2 (WGC/D3D11/MediaFoundation H264/WebRTC, Android decode and cursor-follow zoom) depends on real M1.

## Architecture decision / explicit trust boundaries
- Target: private-interface-bound host native encrypted transport, Android native client independently authenticating pinned host identity, host verifying locally enrolled phone public key in an authenticated handshake, monotonic host-owned association epoch + transcript/channel binding. Native networking and crypto materials NEVER originate from renderer, QR, JSON or signaling; no native authority is minted by proof alone.
- Existing dependency graph cannot supply a verified DTLS exporter/certificate fingerprints/SCTP stream as of baseline. Do NOT manufacture a synthetic producer or accept ordinary TCP as production. A revised TLS1.3/mutual-auth channel contract is acceptable ONLY if both native implementations actually observe and verify the same association, pins, independent phone proof and epoch, with threat-model review and tests. Any unreviewed rustls/WebRTC/Android provider acquisition or permission addition is an owner decision gate, not silent scope.
- First executable production-safe slice should remove a real native integration blocker (e.g. verified actual private-interface native bind/source observation with NO plaintext acceptance and full deny-only enforcement; or a genuine reviewed TLS backend behind an inaccessible guarded admission boundary); tests must cover hostile/nonprivate/loopback/public, reconnect, stale, revocation, early/late, failed cleanup. Do not repeat Ed25519-alone fixture work.
- In absence of a viable cryptographic backend, implement one substantive integrated, source-tested native security seam with no listener/consent/input enrollment and document precise dependency gate. Keep Kotlin bridge reject-only and Tauri DisabledInput until true native producers and OS-local grant exist.
- Host consent requires a separate OS-owned interactive device- and session-scoped native callback with explicit lease, desktop lock/revoke and partial SendInput release debt. A WebView button / Tauri command NEVER grants it.

## Owned non-overlapping work
- Orchestrator: ONLY NEW workplan/R17-{EXECUTION-PLAN,TASKS,WORKLOG}.md, integration interface review, status/QA/commit receipts. Optional apps/windows/** after disjoint claim preflight and genuine host transport interface, NOT prematurely.
- Potential Worker HOST: ONLY crates/host/** + NEW workplan/workers/R17-HOST.md. Implement concrete native-host private LAN/secure transport backend prerequisite, native source tests. No change to Windows, apps/mobile, root lockfile, protocol, other docs.
- Potential Worker ANDROID: ONLY apps/mobile/** + NEW workplan/workers/R17-ANDROID.md. Investigate verified Android/JCA TLS provider and build/toolchain; implement independently useful safe native bridge/secure-peer prerequisite, tests. INTERNET permission or SDK/wrapper installation requires explicit owner authorization; never silently add.
- Deploy max TWO children, sequentially only after own workspace/snapshot/claims, native readiness and successful first child connectedAt. Each child uses fresh host injected task+instance+epoch and same MK059 connector; no successors without owner authorization.

## Validation / acceptance gates
A1 ACTUAL protected transport: host-observed native cryptographic association with private LAN binding, pinned host, native exporter/cert/stream/epoch or accepted secure equivalent + matching Android verification; BLOCKED until integrated native runtime evidence.
A2 Host confirms separately locally enrolled phone key proof, replay/forged/stale/renewal attacks; BLOCKED for live transport.
A3 Windows OS-owned interactive device/session-local consent before guarded input; BLOCKED.
A4 Fuzz/negative native reconnect, expiry, revoke, lock/background, race, incomplete cleanup/partial key-up debt; BLOCKED until integration native tests.
A5 Official pinned Gradle 9.4.1 BAT/JAR, SDK platform37/build-tools37/NDK28.2.13676358 + successful Kotlin JUnit/assemble; BLOCKED per R16 termrun_f861cd90114c4507a7afe1d5014626ca (exit1).
A6 Physical Win11+Android A01–A17, first verified unsigned EXE/debug APK, M2 streaming/latency; BLOCKED. Commands are owner-run after approval, never artifact proof.
- QA: precise focused Rust fmt/check/test/clippy and Android TS typecheck/lint/test plus Kotlin JUnit only if toolchain available; preserve run IDs/exit and missing proof. Governed exact-file Git commits/push, verify origin/main 0/0, protect five inherited paths.
