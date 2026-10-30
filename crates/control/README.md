# Zoomouse M1-E — isolated input executor foundation

**Status: SOURCE-ONLY / NOT ACTIVE.** The public API in `zoomouse_control::policy` is pure and produces no OS side effects. Windows SendInput is a private module; the private executor has no network handler, public constructor or exported native injector. No DataChannel handshake, DTLS exporter, peer verification, actual physical Windows injection, session binding, or Android integration is enabled.

## Trust boundary / handoff

- M1-D-LIVE must supply an independently authenticated WebRTC DataChannel (DTLS certificate/fingerprint verified, exporter/session binding and device key mapped to current AuthenticatedPeer), not a JSON `deviceId` or a pairing-intent string. Pairing TCP alone is not a secure DataChannel.
- M1-C/host must construct the native adapter **behind** `crates/host/src/boundary.rs::HostSession<InputBackend>`. On each frame, host verifies OS-observed DesktopContext, current local consent lease and authenticated peer/session. Then host must enforce packet byte length <= 4096 (from actual encoded bytes), strict wire v1 schema, monotonic sequence starting at 1 with <=64 advance, <=5s stale and +2s future skew, 90 events/60 move/5 text per second. Pure policy preview never grants authority; commit state only after OS emission. Raw DataChannel and public IPC must not call native SendInput directly.
- On disconnect/revoke/lock/suspend/expiry/emergency stop/native failure, host calls unconditional release_all and terminates the old frame epoch. If key/button release fails, session stays revoked and requires local manual recovery; no promise of guaranteed OS cleanup is possible after a failed SendInput call.
- Key strategy: only layout-invariant navigation/modifier/F keys; physical KeyA/DigitN/Numpad/Meta blocked pending native keyboard-layout resolution. Unicode text uses KEYEVENTF_UNICODE, not clipboard; control chars other than newline/tab disallowed.
- Native adapter additionally rejects Session 0, wrong/non-console Windows session, non-Default input/current desktop, unavailable input desktop. Desktop switches can race with SendInput and require native suspend/lock event wiring before activation.

## Verification

Run `cargo fmt --manifest-path crates/control/Cargo.toml -- --check`, `cargo check --manifest-path crates/control/Cargo.toml`, `cargo test --manifest-path crates/control/Cargo.toml`, `cargo clippy --manifest-path crates/control/Cargo.toml --all-targets -- -D warnings`. This crate is intentionally standalone and has no root Cargo workspace edits. Windows native compilation must be independently checked on the Windows toolchain.

**M1 physical A01–A17: NOT RUN.** No APK, Windows installer, WebRTC DataChannel or transport encryption is asserted.
