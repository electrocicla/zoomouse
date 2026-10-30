# Zoomouse

**LAN-first mobile remote control with cursor-follow screen zoom.**

Zoomouse is designed as an Android remote (React Native + strict TypeScript) paired with a user-session Windows 11 host (Rust media/control engine + Tauri 2 settings/tray UI). Its signature feature is a low-latency, privacy-scoped video viewport that follows the Windows cursor.

## Current status

**Architecture and interactive UI preview only (Phase 0).** The preview is intentionally simulated. It does not discover actual PCs, inject mouse or keyboard input, capture the desktop, create a Windows service, or stream real video. Never interpret preview status values as hardware metrics.

## Requirements

- Node.js 22+, pnpm 10
- For future Android builds: Android Studio, Android SDK, JDK and React Native toolchain
- For future Windows builds: Rust, MSVC C++ build tools, WebView2 runtime, Windows 11 SDK

## Run UI preview

```powershell
pnpm install
pnpm dev
```

Open the URL printed by Vite. Use the navigation and streaming/zoom toggles to review the intended UX.

## Validate

```powershell
pnpm check
```

## Repository layout

- `apps/preview`: working React/Vite prototype for UX validation only.
- `packages/protocol`: versioned Zod input, zoom and pairing contracts with strict types/tests.
- `apps/mobile`: Android React Native implementation milestone (not yet scaffolded).
- `apps/windows`: Windows 11 Tauri 2 tray UI milestone (not yet scaffolded).
- `crates/host`: user-session Rust engine milestone (not yet scaffolded).
- `docs`: architecture, implementation roadmap, security rules, reference screen inventory.

## Delivery priorities

1. **LAN pairing and input** with explicit Windows consent, mDNS and direct IP fallback.
2. **Cursor-follow capture, GPU crop and hardware-encoded WebRTC video** within the authorized display/window.
3. **Keyboard, multimedia, per-device settings and reconnection.**
4. **Internet/mobile network access** via authenticated signaling + TURN, after LAN quality and security gates.

See [architecture](docs/ARCHITECTURE.md) and [roadmap](docs/ROADMAP.md).

## License

MIT; see LICENSE.
