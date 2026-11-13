# R16 scoped implementation / acceptance register
| ID | Priority | Initial state | Owners and measurable exit criteria |
|---|---|---|---|
| R16-PLAN | P0 | SOURCE REPORT PUBLISHED | Parent authored actual call graph, security boundary, ownership and QA evidence; final review 2026-11-13. |
| R16-HOST-OBS | P0 | SOURCE HARDENING PUBLISHED; LIVE TRANSPORT BLOCKED | Worker A strict Ed25519, one-use clock rejection; 71 unit+1 doctest, fmt/check/clippy exit0; source 3fa7c0fdbd2a, report receipt 1f51cd4f7e64. Genuine DTLS producer remains absent. |
| R16-ANDROID-IDENTITY | P0 | KOTLIN SOURCE PUBLISHED; NATIVE JUNIT/APK BLOCKED | Worker B JCA Ed25519 ephemeral in-memory one-shot proof, verified Rust 180-byte fixture in unexecuted JUnit, native lifecycle abort; TS typecheck/lint 0, 114 Vitest+5 readiness pass; preflight exit1; 2ccf24e4bb20 published. No real channel. |
| R16-WIN-OS-CONSENT | P0 | BLOCKED BY AUTHENTICATED CHANNEL AND ENROLLMENT | Parent optional later; only OS-local physical decision may mint short-lived scoped grant, not renderer IPC. Tauri currently DisabledInput. |
| R16-PROTOCOL | conditional | READ ONLY | Change wire only for demonstrated incompatibility, never to fake encrypted transport. |
| R16-M1-PHYSICAL | P0 OWNER | NOT RUN | A01-A17 require real Android/Win11, APK/EXE, authenticated LAN, local approval, input/revocation/performance evidence. |
| R16-M2-MEDIA | P1 | DEFERRED | Hardware WGC/D3D11/MF H264/WebRTC/Android decode/cursor-follow proof and metrics only after M1. |
| R16-M3/M4 | later | DEFERRED | UX/platform parity and Internet TURN require separately accepted secure M1/M2. |

Inherited foreign five dirty paths are protected. No artifact or physical readiness implied by tests/commits. Workers must record exact commands, runIds, exit codes and governed push SHA in their NEW worker files; parent reconciles outcome and updates register.

## R16 final source-only acceptance (2026-11-13 CLST)
Host and Android source branches were committed through governed exact-file Git and both verified main == origin/main ahead0 behind0 before parent reconciliation. Five inherited foreign dirty paths remain unchanged and unstaged. **M1 READY = NO**: no real DTLS/SCTP/native peer exporter, cert/stream association, Windows OS-owned grant, authenticated LAN input or physical A01–A17. Android Gradle wrapper/SDK and controlled external transport dependencies remain owner decisions; no EXE/APK or WGC/H264/decoder acceptance.
