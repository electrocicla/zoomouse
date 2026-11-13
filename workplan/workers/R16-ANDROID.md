# R16-ANDROID — Native Kotlin one-shot Ed25519 proof, admission DENIED

Date 2026-11-13 CLST. Independent task deployment-child-3433f54a8e5d39f2387145f369c07624; instance web-child-3433f54a8e5d39f2387145f369c07624; lifecycle ebe00e56-a6fc-47aa-9a27-99e357048124. Authorized Zoomouse rootIndex13, github.com/electrocicla/zoomouse main. Starting HEAD=origin/main 662f53a5a2e7 (ahead0 behind0). Disjoint HOST worker published 3fa7c0fdbd2a and receipt 1f51cd4f7e64 (origin/main 0/0) before Android QA.

## Source implemented

- NEW apps/mobile/android/app/src/main/java/com/zoomouse/mobile/SecureLanPairingProof.kt: Kotlin-internal, JCA Ed25519 ephemeral process key identity with CSPRNG nonce, SHA256 device ID and exact Rust pairing::transcript bytes: ASCII domain ZOOMOUSE-LAN-PAIR-ED25519-V1 plus NUL + host key32 + phone key32 + request16 + host nonce32 + phone nonce32 + address family4 + private IPv4 octets4 + big-endian source port2 = 180 bytes. Verifies signed host offer against QR-pinned host key, transcript nonce, request, observed phone endpoint and device ID. One pending one-shot proof with 60s monotonic TTL, attemptId and strictly increasing native association generation. Consumes challenges before verification, rejects wrong association/endpoint/key/signature/nonce, clock rollback, expiry/replay, invalid private source and unavailable Ed25519 provider. Never downgrades crypto or opens a socket.
- MODIFIED apps/mobile/android/app/src/main/java/com/zoomouse/mobile/SecureLanModule.kt: dormant Kotlin-only verifier object; abortAll on background/destroy/invalidate and abortAttempt on exact close. React Native openSecureChannel STILL ALWAYS REJECTS; no new JS permission or private-key export, no network, input or capture route.
- NEW apps/mobile/android/app/src/test/java/com/zoomouse/mobile/SecureLanPairingProofTest.kt: JUnit4 SOURCE-ONLY tests for Rust published 180-byte vector, deterministic host/phone Ed25519 signatures, device ID, bad signature, wrong host/request/nonce/endpoint/device ID, wrong generation, replay, aborted/late callback, background/destroy clearing, expired challenge, absent JCA Ed25519, WAN/loopback/broadcast/noncanonical addresses. Existing SecureLanAttemptFenceTest.kt covers stale IDs, pause/destroy, repeated close and invalid ports. KOTLIN TESTS UNEXECUTED because Android toolchain is blocked.

## Threat analysis and production denial

No Android native DTLS/SCTP/WebRTC encrypted connection, independently obtained DTLS exporter/certificate fingerprints/SCTP stream/association producer, encrypted live host socket or native Windows local consent grant exists. The current AndroidManifest does not request INTERNET. The new internal begin method has NO production caller; its contract requires a trustworthy native observation and cannot interpret untrusted JSON/QR as an association. A valid Ed25519 pairing signature or source-only test can never create remote input authority. No plaintext TCP, fake producer, WAN listener, certificate export to JS or consent bypass was introduced.

Current ephemeral KeyPairGenerator Ed25519 is JCA-provider-backed SOFTWARE-ELIGIBLE, NOT AndroidKeyStore, hardware-backed, device attested or persisted. minSdk24 does not guarantee a runtime Ed25519 JCA provider. Missing provider rejects; no algorithm fallback. Actual Android crypto provider support and native pairing tests are NOT verified. Host local enrollment is separate; Rust HOST proofs remain subordinate to native encrypted channel + desktop consent. A01–A17 physical tests, APK, native signing/install and input remain NOT RUN.

## Governed host test receipts — terminal agent-term-1794586652649-9, I:\Zoomouse

| Command | runId | Exit | Scope |
| --- | --- | ---: | --- |
| pnpm --filter @zoomouse/mobile typecheck | termrun_a990221624974ed0b1b9e8a0783b93a0 | 0 | TS typecheck |
| pnpm --filter @zoomouse/mobile lint | termrun_8e10fda6a2a94bd8ae64dd51b85e5d69 | 0 | ESLint |
| pnpm --filter @zoomouse/mobile test | termrun_091058d7ddfa4529b80b2e9f20231378 | 0 | 5 Node readiness + 114 Vitest, not Kotlin |
| pnpm --filter @zoomouse/mobile android:preflight | termrun_f861cd90114c4507a7afe1d5014626ca | 1 | Missing official gradlew/gradlew.bat/verified Gradle9.4.1 wrapper JAR, Android SDK Platform37 and NDK28.2.13676358; earlier SDK root/BuildTools37 also failed. NO build attempted. |
| Get-Command kotlinc | termrun_27818ea697ac45c094e72742d316ca0d | 0 | KOTLINC_NOT_INSTALLED |

All above are host-confirmed tracked terminal runId and exact exit code. The native Kotlin JUnit files were NOT compiled or executed.

## Owner authorization / dependency gate

Require separately authorized official Gradle 9.4.1 wrapper scripts + JAR, legitimate SDK root/Platform37/BuildTools37/NDK28.2 setup; an audited real Android DTLS/SCTP producer with matching Rust host native transport, independent peer cert/exporter/stream and increasing association evidence, and approval for any external native transport dependencies or INTERNET permission changes. Only thereafter owner-authorized native build/sign/install and Android-to-Windows physical lifecycle/consent A01–A17. Do not unlock app until proven.

## Exact scoped Git publication

ONLY these four paths are authorized to stage: SecureLanModule.kt; SecureLanPairingProof.kt; SecureLanPairingProofTest.kt (full paths above), and workplan/workers/R16-ANDROID.md. Five inherited foreign dirty paths are strictly untouched: .gitignore, ROOT package.json, pnpm-lock.yaml, workplan/workers/R6-WIN.md, workplan/workers/R13-ANDROID.md. Specialized Git stage/commit/push only. Final SHA and origin/main receipt will be messaged to orchestrator.
