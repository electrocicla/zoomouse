# Zoomouse Rust host — M1-C foundation

This crate is a pure Rust, dependency-free, **deny-by-default** user-session security seam. It is not a Windows service, device pairing server, media engine, or Win32 input injector.

## Modules

- `boundary.rs`: opaque authenticated transport identity, OS-observed interactive desktop state and the only remote-input dispatch gateway. Constructors for trusted identities are crate-private; untrusted JSON/IPC is not a constructor.
- `consent.rs`: host-local pending → approved/denied/revoked state, 10-minute maximum lease, session/device/transport binding, expiration and local approval rules. No remotely callable grant method.
- `input.rs`: abstract native input dispatch/release interface and a production-default disabled backend.
- `error.rs`: explicit failure reasons, including an unrecoverable cleanup warning.

`HostSession::dispatch` verifies an approved authenticated peer and desktop before touching the backend. Disconnect, emergency stop, lock, expiry or native dispatch failure revokes first and attempts `release_all`. A cleanup failure leaves the session revoked and surfaces `CleanupFailed`.

Local approval is deliberately **unreachable in production** until M1-D supplies a real authenticated pairing adapter, verified Windows session identity and a trusted native consent callback. These internal seams are exercised by pure Rust unit tests; test fixtures do not constitute actual authentication. The shipped `DisabledInput` backend refuses all native events.

## Focused check

From the repository root, once Rust is installed: `cargo test --manifest-path crates/host/Cargo.toml`. This only validates the pure state machine. The Tauri application has its own manifest at `apps/windows/src-tauri/Cargo.toml`.

## Required subsequent work

M1-D must implement authenticated device and DTLS/session binding, actual OS desktop observation, private LAN discovery, request presentation and local consent identity; M1-E must add a validated packet/rate/replay gate and audited Windows 11 input backend, held-input tracking and fail-safe cleanup. Neither workstream may expose an arbitrary shell/exec/filesystem interface, accept remote consent, use elevation or dispatch into Session 0. M1-F must perform physical Windows 11↔Android LAN E2E and real revocation/cleanup observation.
