# Zoomouse Rust host — M1-C foundation

This crate is a Rust-only, **deny-by-default** user-session security seam. The M1-D pairing substrate adds Ed25519 (ed25519-dalek), SHA-256 and OS CSPRNG dependencies, but no listener, input or capture authority. It is not a Windows service, device pairing server, media engine, or Win32 input injector.

## Modules

- `boundary.rs`: opaque authenticated transport identity, OS-observed interactive desktop state and the only remote-input dispatch gateway. Constructors for trusted identities are crate-private; untrusted JSON/IPC is not a constructor.
- `consent.rs`: host-local pending → approved/denied/revoked state, 10-minute maximum lease, session/device/transport binding, expiration and local approval rules. No remotely callable grant method.
- `input.rs`: abstract native input dispatch/release interface and a production-default disabled backend.
- `error.rs`: explicit failure reasons, including an unrecoverable cleanup warning.

`HostSession::dispatch` verifies an approved authenticated peer and desktop before touching the backend. Disconnect, emergency stop, lock, expiry or native dispatch failure revokes first and attempts `release_all`. A cleanup failure leaves the session revoked and surfaces `CleanupFailed`.

Local approval is deliberately **unreachable in production** until M1-D supplies a real authenticated pairing adapter, verified Windows session identity and a trusted native consent callback. These internal seams are exercised by pure Rust unit tests; test fixtures do not constitute actual authentication. The shipped `DisabledInput` backend refuses all native events.


## M1-D LAN/pairing security substrate (not an active server)

- `lan.rs` only admits private RFC1918 IPv4 / RFC4193 IPv6 ULA endpoints (nonzero port), using a validated constructor with private fields so other modules cannot forge a `PrivateLanEndpoint` via a public struct literal. It emits a versioned `zoomouse://pair/v1` descriptor with **the complete host Ed25519 public key** for a locally displayed QR. It includes no password/token. Manual IP and mDNS-derived names are *locators only*; scan of the native Windows display or independently verified fingerprint must pin the public key. It does not render QR, resolve addresses, register mDNS, bind a network socket, or install a Windows Firewall rule.
- `pairing.rs` generates a fresh host Ed25519 key from the OS CSPRNG per process lifetime; validates phone public-key claims, signs a domain-separated challenge over both public keys, both nonces, request ID, private source IP/port, enforces one-use proof (invalid, expired and wrong-source proofs consume the request) with 60s TTL and a hard cap of 16 outstanding candidates. A valid phone Ed25519 proof queues a **candidate**, not a connection grant. Local enrollment requires a trusted desktop context and a separate local native decision, expires within 10 minutes, and can be revoked. No state is restored after restart.
- `HostSession` and `ConsentController` remain separate and deny input. Local *pairing enrollment* by itself NEVER manufactures `AuthenticatedPeer`, approves the separate HostSession input grant, or verifies an encrypted network transport. The production Tauri shell retains `DisabledInput` and no pairing-approval invoke command; mDNS and QR are not advertised as operational.
- M1-A/M1-B wire-contract request: the current `pairing.intent` transports a claimed ID and nonce **without** public-key proof. Define a separate versioned Ed25519 hello/challenge/proof protocol with deterministic transcript encoding and host public-key pinning; device ID must derive from the key, not a mutable display name. Actual incoming peer IP must come from the transport, not untrusted JSON. The current Rust substrate is an internal draft until interoperable TS/Android fixtures are reviewed.
- M1-E integration gates: authenticate a real encrypted DTLS/WebRTC channel, cryptographically bind that channel key/transcript to the approved phone key, use a fresh per-session transport binding ID and current consent generation, and call trusted OS-session host consent **before** native input. Revoke must invalidate both pairing and HostSession and attempt native key/button release. Require a verified private interface binding and explicit firewall scope before registering mDNS, serving QR/accepting connections, or claiming LAN availability.
- Out of scope and unverified: listener/mDNS responder, QR camera/parser, persistent host key management, cross-language protocol fixtures, secure channel, local native consent presentation, Windows network/capture/input E2E. No UAC or Session 0 semantics are introduced.

Run: `cargo test --manifest-path crates/host/Cargo.toml`, `cargo fmt --manifest-path crates/host/Cargo.toml --check` and `cargo check --manifest-path crates/host/Cargo.toml`; these validate the pure substrate only.

## M1-D-LIVE — private TCP pairing adapter (SOURCE ONLY)

`src/pairing_tcp.rs` is a **crate-private**, never-started listener foundation. The existing Tauri shell does not import or invoke it. A future Windows host-local owner must explicitly call the constructor after validating OS network interface, LAN firewall policy, interactive desktop and a reviewed encrypted transport. Nothing currently binds a production port or advertises mDNS/QR operationally.

- `PairingTcpListener::bind` accepts only a validated, specific RFC1918/ULA interface and confirms that the operating system bound exactly the requested address; wildcard, loopback, link-local, CGNAT or public endpoints are denied by `PrivateLanEndpoint`. Address matching alone **does not** rule out VPN/tunnel exposure; firewall/route/interface provenance is a separate blocked gate.
- The nonblocking `accept_once` gets the peer from `TcpListener::accept`, never from wire JSON. On each new connection it enforces 8 attempts/IP/60s with 64 tracked IPs, 2048-byte UTF-8 newline-delimited JSON frames, strict `version=1`/types/unknown-field rejection, fixed lowercase hex, 3s absolute per-frame read deadlines, 3s socket write timeout and deterministic socket shutdown on all paths. Rust registry retains the preexisting maximum 16 outstanding challenges and 60s TTL.
- `pairing.phone-hello` invokes native Ed25519 challenge signing. `pairing.host-challenge` returns the exact **OS-observed socket endpoint** in the signed transcript. `pairing.phone-proof` must return over the same socket and validate the original request ID/key/source; a valid proof yields ONLY a `LocalPairingRequest` for future native display, never input/capture permission. Incorrect signature or source consumes the challenge and cannot replay.
- The bound QR helper can derive only the host-key locator from an actual bound private TCP socket + live signing key; it intentionally does not advertise the legacy UDP mDNS service type. **No mDNS registrar or native QR renderer** is installed. Manual IP and an advertised host key are locators, never local approval; mobile must verify its independently pinned host key/signature with native crypto.
- **Do not turn this plaintext TCP adapter into a shipping user-facing listener.** It has no TLS/DTLS channel confidentiality, transport-to-phone-key cryptographic binding, OS-trusted local decision UI, Windows firewall/interface attestation or end-to-end revocation/input cleanup integration. It is not a secure control DataChannel. There is no native input, desktop capture, shell or elevation endpoint in the adapter.
- Synthetic Rust tests cover v1 JSON → challenge → Ed25519 proof and one-use burn, wrong-source rejection, invalid/extra/oversized input and rate caps; physical Android↔Windows and A01–A17 are **NOT RUN**. A native private-IP bind is not claimed from loopback test fixtures.

## Focused check

From the repository root, once Rust is installed: `cargo test --manifest-path crates/host/Cargo.toml`. This only validates the pure state machine. The Tauri application has its own manifest at `apps/windows/src-tauri/Cargo.toml`.

## Required subsequent work

M1-D must implement authenticated device and DTLS/session binding, actual OS desktop observation, private LAN discovery, request presentation and local consent identity; M1-E must add a validated packet/rate/replay gate and audited Windows 11 input backend, held-input tracking and fail-safe cleanup. Neither workstream may expose an arbitrary shell/exec/filesystem interface, accept remote consent, use elevation or dispatch into Session 0. M1-F must perform physical Windows 11↔Android LAN E2E and real revocation/cleanup observation.
