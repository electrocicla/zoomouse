# M1 Physical Acceptance Matrix — LAN input

All cases start **NOT RUN**. This plan is not evidence of APK, Windows host, secure LAN transport or a successful Android/Win11 E2E. Record Android build/device, Windows 11 release/driver, LAN type, code SHA and test logs before updating statuses.

| ID | Scenario | Acceptance gate | Status |
|---|---|---|---|
| A01 | Android discovers Windows via LAN mDNS | Correct host identity, no cloud dependency, no ghost hosts | NOT RUN |
| A02 | Multicast blocked: manual IP | Same authenticated approval flow as mDNS | NOT RUN |
| A03 | QR pairing | Version-bound bootstrap, no embedded reusable long-lived secrets | NOT RUN |
| A04 | Unknown device attempts input | No injected input or frame disclosure until verified local consent | NOT RUN |
| A05 | Windows grants a specific device | Cryptographic device/session binding, visible tray authorization and expiry | NOT RUN |
| A06 | Windows rejects/revokes device | Existing session invalidated; all held keys/buttons released | NOT RUN |
| A07 | Move, click, right-click and drag | Reproducible native cursor/button effects; bounded deltas | NOT RUN |
| A08 | Vertical/horizontal scrolling | Direction, sensitivity, rate and ordering correct | NOT RUN |
| A09 | Text/keyboard modifiers/key-up | Actual Win32 keyboard behavior; no stuck keys | NOT RUN |
| A10 | Replay/reorder/stale/oversize packets | Rejected by host without side effects; bounded queues | NOT RUN |
| A11 | Disconnect/reconnect/sleep/wake | Truthful states, bounded retries, no auto-grant | NOT RUN |
| A12 | Second non-approved phone | Independent identity; cannot borrow first phone's grant | NOT RUN |
| A13 | Emergency stop and tray revoke | Immediate input stop and key/button cleanup | NOT RUN |
| A14 | Secure Desktop/UAC or locked screen | No privileges beyond interactive user session | NOT RUN |
| A15 | Public Wi-Fi or Internet-facing network | No accidental WAN listener, private LAN firewall boundary | NOT RUN |
| A16 | Latency/resource telemetry | Measure actual pointer latency p50/p95, errors and CPU/Battery; compare goals without inventing | NOT RUN |
| A17 | APK and Windows packaging | Reproducible install and native build logs bound to SHA | NOT RUN |

Pure protocol tests and jsdom preview tests do not satisfy physical gates. Commands: `pnpm check`; run native Android Gradle and Rust Cargo/Tauri check once scaffolds exist. Attach results to per-worker logs and to the parent worklog. Fail closed if approval/session identity is unknown; no Windows service Session 0; no public Internet listener until M4 security approval.
