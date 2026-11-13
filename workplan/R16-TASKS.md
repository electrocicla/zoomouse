# R16 scoped implementation / acceptance register
| ID | Priority | Initial state | Owners and measurable exit criteria |
|---|---|---|---|
| R16-PLAN | P0 | IN PROGRESS | Parent only 3 NEW R16 docs, actual dependency graph, source-work QA and publication. |
| R16-HOST-OBS | P0 | PENDING | Worker A: native host crypto/transport authority boundary, real backend evidence feasibility, positive+negative tests, one scoped published commit; NEVER claim fixture is native DTLS. |
| R16-ANDROID-IDENTITY | P0 | PENDING | Worker B: actual Kotlin native secure connection/identity feasibility, native cancel/revoke/race safety, source tests, Android preflight gate receipt and scoped publication. |
| R16-WIN-OS-CONSENT | P0 | BLOCKED BY AUTHENTICATED CHANNEL AND ENROLLMENT | Parent optional later; only OS-local physical decision may mint short-lived scoped grant, not renderer IPC. Tauri currently DisabledInput. |
| R16-PROTOCOL | conditional | READ ONLY | Change wire only for demonstrated incompatibility, never to fake encrypted transport. |
| R16-M1-PHYSICAL | P0 OWNER | NOT RUN | A01-A17 require real Android/Win11, APK/EXE, authenticated LAN, local approval, input/revocation/performance evidence. |
| R16-M2-MEDIA | P1 | DEFERRED | Hardware WGC/D3D11/MF H264/WebRTC/Android decode/cursor-follow proof and metrics only after M1. |
| R16-M3/M4 | later | DEFERRED | UX/platform parity and Internet TURN require separately accepted secure M1/M2. |

Inherited foreign five dirty paths are protected. No artifact or physical readiness implied by tests/commits. Workers must record exact commands, runIds, exit codes and governed push SHA in their NEW worker files; parent reconciles outcome and updates register.
