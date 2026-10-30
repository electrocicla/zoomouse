# Zoomouse — ADR-0001, LAN-first architecture

## Product boundary
Android React Native remote controls a Windows 11 *interactive user-session* agent. v1 LAN-only; internet/mobile network access is a later release. Never implement capture in a Session 0 Windows service.

## Technical choices
- `apps/mobile`: React Native New Architecture, strict TypeScript, Android native video rendering.
- `apps/windows`: Tauri 2 + React + TypeScript for tray, consent, device configuration.
- `crates/host`: Rust user-session engine with Windows APIs for discovery, pairing, input and GPU media capture.
- Windows.Graphics.Capture → D3D11 GPU crop/scale → hardware H.264 via Media Foundation → WebRTC to Android.
- WebRTC DataChannel for trusted pointer, keyboard, scroll, shortcuts and control; separate channels for high-rate lossy move vs reliable key-up/disconnect.
- mDNS host discovery, IP fallback and QR onboarding. No cloud dependency for LAN.
- Shared Zod schemas with explicit protocol versions; Rust must implement wire-equivalent validation and test vectors.

## Data flow
```text
Android RN ──[mDNS + direct IP + QR]──► Windows host
Android RN ──[approved WebRTC DataChannel]──► session input executor
Android RN ◄──[WebRTC DTLS/SRTP H.264 video]── Windows user-session agent
                                  WGC source → GPU crop → encoder
                                   ▲
                     cursor position + DPI transforms
Tauri tray ◄── consent / revoke / capture indicator ──► Windows host
```

## Cursor Follow Zoom (differentiator)
1. The user selects a Windows display/window capture source. Respect operating-system consent and protected content.
2. Convert GetCursorPos global desktop coordinates to capture-local coordinates, handling negative monitor origins, per-monitor DPI, rotation and monitor topology.
3. Calculate output-aspect crop rectangle using zoom factor 1.0–4.0, clamp to source bounds and apply time-based smoothing (independent of frame rate).
4. Crop **on GPU** and encode reduced viewport. Do not repeatedly transfer full frames to CPU.
5. Separate toggles: streaming, zoom and following. Zoom OFF renders entire selected source; follow OFF freezes crop center; fixed-crop mode is optional.
6. Avoid double cursor rendering, preserve input-to-desktop coordinate mapping, and reset transform safely on display size/topology changes.
7. Maintain source selection, visible capture indicator, emergency stop and a stable disconnected/reconnecting UI.

## Security invariants
- No remote actions before explicit local approval and authenticated device/session.
- Pair with per-device identity, key confirmation, limited grants and revocation; reject replay and stale packet sequences.
- Trusted user-session, private firewall scope; no elevation/UAC secure-desktop remote control.
- Reject arbitrary shell/file/process execution; default clipboard and recordings OFF.
- Enforce message rate/size bounds, pointer coalescing, key-release cleanup on disconnect.
- No Internet listener until separate threat model and relay architecture are validated.

## Acceptance gates
M1: genuine Android-to-Windows 11 LAN input on physical devices; mDNS/IP fallback, approvals and recovery tested.
M2: genuine low-latency video and follow-cursor zoom on mixed-DPI/multi-monitor setups, hardware encoder fallback tested.
M3: UX and settings parity verified; no incomplete buttons.
M4: mobile internet via TLS signaling + ICE/TURN, CGNAT tests, revocation and cost controls.

## Alternatives
React Native Windows offers UI reuse, but Windows capture/input still need native modules. A Tauri 2 Rust host provides smaller operational separation. All performance claims require real device measurements.

References:
- https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture
- https://tauri.app/learn/system-tray/
- https://microsoft.github.io/react-native-windows/support/
