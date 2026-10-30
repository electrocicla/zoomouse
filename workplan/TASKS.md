# Zoomouse — orchestrator task list

Update statuses only against verified work. The parent owns this file; workers write independent `workplan/workers/<task-id>.md` and Agents Global semantic logs.

| ID | Priority | Phase | Description / observable acceptance | Owner | Status | Dependencies |
|---|---|---|---|---|---|---|
| ORCH-001 | P0 | Coordination | Plan, scoped parallel contracts, on-disk worklog, validated/pushed docs | Parent | DONE (0dc3f69) | M0 |
| ORCH-002 | P0 | Coordination | Spawn verified ChatGPT integrated-browser workers; capture IDs and connector; no duplicate Send | Parent | DONE (A/B/C connected) | ORCH-001 |
| M1-A | P0 | M1 | Versioned protocol and replay/authorization rules + defensive tests, fixed vectors | Child A deployment-child-f07a00ae2e83139a2c651509bcced8c0 | ACTIVE | M0 |
| M1-B | P0 | M1 | RN Android scaffold with strict typing, correct permissions UX; Android compilation evidence | Child B deployment-child-2731de29d52bc55a0d490fb10df4b550 | ACTIVE | M0 |
| M1-C | P0 | M1 | Tauri/Rust Windows session-host scaffold, explicit consent shells; native build evidence | Child C deployment-child-5dd95eac0f20ed757e1075ffb1ae2f5a | ACTIVE | M0 |
| M1-D | P0 | M1 | Secure identity, mDNS/manual IP/QR, pairing, revocation, firewall LAN scope | Unassigned | BACKLOG | A,B,C |
| M1-E | P0 | M1 | DataChannel trustworthy dispatch, click/drag/scroll/keyboard + release cleanup | Unassigned | BACKLOG | D |
| M1-F | P0 | M1 | Physical Android↔Win11 E2E, security/latency and packaging gate | Unassigned | BACKLOG | E |
| M2-A | P0 | M2 | Consent-bound WGC source selection in user session | Unassigned | BACKLOG | M1-F |
| M2-B | P0 | M2 | GPU crop/scale and H.264 over WebRTC to Android | Unassigned | BACKLOG | M2-A |
| M2-C | P0 | M2 | Cursor-follow independent zoom toggles, DPI/edge clamps, real-device verification | Unassigned | BACKLOG | M2-B |
| M3-A | P1 | M3 | Input ergonomics, shortcuts, multimedia and settings parity | Unassigned | BACKLOG | M2-C |
| M3-B | P1 | M3 | Accessibility, history, themes/i18n, mobile UX visual QA | Unassigned | BACKLOG | M3-A |
| M4-A | P0 later | M4 | Internet threat model/signaling/TURN and CGNAT validation | Unassigned | BACKLOG | M3-B |

### Mandatory worker completion template

`Task / repo / branch / touched paths / base SHA / changes / checks (commands + result) / risks / unfinished gates / commit SHA / push receipt / Agents Global log phase / parent notified`. No unverifiable DONE.
