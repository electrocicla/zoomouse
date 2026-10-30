# Zoomouse — orchestrator task list

Update statuses only against verified work. The parent owns this file; workers write independent `workplan/workers/<task-id>.md` and Agents Global semantic logs.

| ID | Priority | Phase | Description / observable acceptance | Owner | Status | Dependencies |
|---|---|---|---|---|---|---|
| ORCH-001 | P0 | Coordination | Plan, scoped parallel contracts, on-disk worklog, validated/pushed docs | Parent | DONE (0dc3f69) | M0 |
| ORCH-002 | P0 | Coordination | Spawn verified ChatGPT integrated-browser workers; capture IDs and connector; no duplicate Send | Parent | DONE (A/B/C connected) | ORCH-001 |
| M1-A | P0 | M1 | Versioned protocol and replay/authorization rules + defensive tests, fixed vectors | Recovery child deployment-child-bbc34cdad3c9a4d00d2198cdc94cd93f | SOURCE PUBLISHED 76ba8fbc; 21 protocol tests PASS; physical wire integration NOT ACCEPTED | M0 |
| M1-B | P0 | M1 | RN Android scaffold with strict typing, correct permissions UX; Android compilation evidence | Recovery deployment-child-bd036def09993f8b675ba7998345ec26 | SOURCE PUBLISHED 435c7ded + docs 37ef9d74; Android strict TS/lint and 22 tests PASS (historical worker receipts); SDK37/Gradle wrapper/APK/physical E2E NOT RUN | M0 |
| M1-C | P0 | M1 | Tauri/Rust Windows session-host scaffold, explicit consent shells; native build evidence | Recovery deployment-child-548f354e81071946b69b92892356e241 | SOURCE PUBLISHED febd71a8 + docs 0a01e8a0; Windows typecheck/Vite, native cargo check/clippy/fmt and 4 Node tests PASS (historical worker receipts); installer/physical E2E NOT RUN | M0 |
| M1-D | P0 | M1 | Secure identity, mDNS/manual IP/QR, pairing, revocation, firewall LAN scope | Recovery child deployment-child-1dc365316ae781182e8dba888e1cdd19 | CRYPTO SUBSTRATE PUBLISHED a79b295 + docs b590b65; Rust 27 tests + 1 doc test, fmt/check/clippy PASS; real LAN listener/mDNS/QR/approval integration PENDING | A,B,C |
| M1-D-WIRE | P0 | M1 | Strict Ed25519 pairing hello/challenge/proof TS↔Rust wire and deterministic fixtures | Child deployment-child-561935b02268766930c7f1c696db2a65 | SOURCE PUBLISHED 6541bee + docs 0bb3f22; strict TS, ESLint and 30 protocol tests PASS; native integration NOT ACCEPTED | M1-A,M1-D |
| M1-D-LIVE | P0 | M1 | Host private-LAN listener, signed discovery/QR, explicit local approval plumbing; no public exposure | Child deployment-child-d2a6e0c454141e1bb786dc98ec3ccfbd | SOURCE PUBLISHED 690ef3d + docs 5f72298; 32 Rust unit tests + 1 doc test, fmt/check/clippy PASS; private TCP adapter NOT enabled in production, local approval/mDNS/QR still pending | M1-D,M1-D-WIRE |
| M1-E | P0 | M1 | DataChannel trustworthy dispatch, click/drag/scroll/keyboard + release cleanup | Child deployment-child-84d4978c2f7d086b095215765c87737e | RECOVERY ACTIVE: new MK059 child deployment-child-9d4424009a7aa817fbc6376961ae30a1 authenticated 2026-10-30T22:18:00Z; crates/control/** and M1-E.md remain uncommitted; prior 7 Rust tests/fmt/check/clippy PASS historical; DataChannel/physical input NOT ACCEPTED | M1-D-LIVE,M1-B,M1-C |
| M1-D-LIVE2 | P0 | M1 | Trusted local approval, per-device/session host admission and revocation integration, fail-closed private LAN | Child deployment-child-a19e9fa8d84027ef663a3eafcef9c6fe | ACTIVE: authenticated MK059 child connectedAt 2026-10-30T22:19:46.795Z; host integration implementation/QA/commit pending; A01-A17 NOT RUN | M1-D-LIVE,M1-D-WIRE |
| M1-F | P0 | M1 | Physical Android↔Win11 E2E, security/latency and packaging gate | Unassigned | BACKLOG | E |
| M2-A | P0 | M2 | Consent-bound WGC source selection in user session | Recovery child deployment-child-4536a5890b6d1a47e477248ab42a9cd0 | SOURCE PUBLISHED 121429f1; 7 Rust tests PASS; physical capture/host integration NOT ACCEPTED | M1-F |
| M2-B | P0 | M2 | GPU crop/scale and H.264 over WebRTC to Android | Child deployment-child-c2e1fd2c6611abc865ec216c3cc954e1 | SOURCE PUBLISHED ab11b160; 8 Rust tests/fmt/check/clippy PASS historical; docs-only M2-B.md remains dirty and no live worker; real D3D11 crop/H.264/WebRTC and physical QA NOT IMPLEMENTED | M2-A |
| M2-C | P0 | M2 | Cursor-follow independent zoom toggles, DPI/edge clamps, real-device verification | Child deployment-child-a72b2445a25c3235d9564d6042475ab6 | SOURCE PUBLISHED 2763ab58021a; 13 Rust tests/fmt/check/clippy PASS historical; M2-C.md worker log remains untracked, original lifecycle retired; native video mapping/physical E2E NOT RUN | M2-B |
| M3-A | P1 | M3 | Input ergonomics, shortcuts, multimedia and settings parity | Unassigned | BACKLOG | M2-C |
| M3-B | P1 | M3 | Accessibility, history, themes/i18n, mobile UX visual QA | Unassigned | BACKLOG | M3-A |
| M4-A | P0 later | M4 | Internet threat model/signaling/TURN and CGNAT validation | Unassigned | BACKLOG | M3-B |

### Mandatory worker completion template

`Task / repo / branch / touched paths / base SHA / changes / checks (commands + result) / risks / unfinished gates / commit SHA / push receipt / Agents Global log phase / parent notified`. No unverifiable DONE.
