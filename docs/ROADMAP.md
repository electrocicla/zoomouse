# Delivery plan — evidence gates

## M0: foundation & interactive preview
- [x] Inspect source Android screenshots and map feature inventory.
- [x] Inspect authorized MK059 workspace `I:\Zoomouse`; design architecture.
- [ ] Initialize Git, create *public* `electrocicla/zoomouse`, commit/push and verify remote (accept only observed evidence).
- [ ] Build pnpm, strict TypeScript contracts, simulated React UI preview, unit tests, execute `pnpm check`.
Exit criteria: CI-ready runnable preview; no claim of real Android/Windows control.

## M1: secure LAN input
- React Native Android scaffold and native host/tray scaffold.
- Device discovery via mDNS plus manual IP; QR pairing, local Windows approval, device ACL and revocation.
- WebRTC control transport; pointer move, scroll, clicks, drag, shortcuts, text and key-up handling.
- Reconnect with bounded backoff; private firewall UX and recovery.
Exit: one Android phone controls mouse/keyboard of approved Windows 11 host on same LAN.

## M2: video + cursor zoom
- User-authorized WGC display/window source; D3D11 crop, Media Foundation hardware H.264, WebRTC decoder Android.
- Screen region follows cursor with DPI-aware transforms, adjustable zoom, independent follow toggle, smooth motion, edge clamping.
- Display topology, sleep, GPU/encoder fallback, frame rates and bandwidth adaptation.
Exit: E2E physical phone and Windows capture, p95 latency benchmarks and multi-monitor correctness.

## M3: reference-image feature parity
- Device history, direct IP, QR, input sensitivities, natural scroll, gyro, left-handed mode, modifier keys, click sound, volume buttons.
- Media/browser/app controls, optional clipboard permission, keyboard buffering, reconnect, screen wake/rotation settings.
- Themes/background, i18n, accessibility, diagnostics, support and help.
Exit: per-setting UX tests and complete device scenarios.

## M4: cellular/internet
- Authenticated signaling, ICE/STUN/TURN, device authorization and data-usage controls; no default open raw ports.
- CGNAT and cellular E2E, replay abuse tests, TURN credential lifecycle and billing visibility.
Exit: approved mobile-data end-to-end with user-facing revocation.

## Initial engineering SLO targets (not measured)
- Pointer-to-action p95 under 60 ms on reliable LAN.
- Cropped video glass-to-glass p95 under 180 ms initially.
- Initial 720p at 30 FPS adaptive; optional 60 FPS profile only after hardware validation.
- Zero silent remote input and zero unauthorized frame access.
Targets can change after real instrumentation.
