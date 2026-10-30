# M1-D-WIRE — additive Rust/TypeScript LAN pairing v1

**Scope:** source-only, untrusted packet and QR parsing. `@zoomouse/protocol/pairing-wire` preserves `PROTOCOL_VERSION = 1`, existing input schemas and `session-policy`. No native listener, trusted crypto, active approval, DTLS integration, or input permission is supplied.

## JSON frames (strict UTF-8 <= 2048 bytes)

All packets have numeric `version:1`, fixed `type` and no extra properties; all binary fields are lowercase hexadecimal without prefix.

- `pairing.phone-hello`: `phonePublicKeyHex` (32 bytes), `phoneNonceHex` (32 nonzero bytes), `displayName` (1–128 UTF-8 bytes, not whitespace-only).
- `pairing.host-challenge`: `requestIdHex` (16), `hostPublicKeyHex` (32), `hostNonceHex` (32), `hostSignatureHex` (64), `deviceIdHex` (32), `observedPhoneEndpoint: {address,port}`. The endpoint is the **host-observed actual socket source**, not an unauthenticated phone declaration.
- `pairing.phone-proof`: `requestIdHex` (16), `phoneSignatureHex` (64).

The existing Rust `PhoneHello`, `HostChallenge` and `prove` use native structs, not a JSON socket adapter. Native Rust must parse equivalent fields, enforce actual source binding, challenge single-use/expiration and max pending count; mobile must sign/verify through trusted native crypto, not TypeScript.

## Exact Ed25519 transcript (Rust `pairing::transcript`)

```text
ASCII("ZOOMOUSE-LAN-PAIR-ED25519-V1") + 0x00
+ hostPublicKey[32] + phonePublicKey[32] + requestId[16]
+ hostNonce[32] + phoneNonce[32]
+ IP family byte (0x04 IPv4, 0x06 IPv6)
+ actual phone source address octets[4|16]
+ source port uint16 big-endian[2]
```

Lengths: IPv4 **180 bytes**; IPv6 **192 bytes**. Signature of host and phone binds these bytes, subject to native Ed25519 verification. The phone must compare the challenge's host key against a key **pinned from independently trusted local QR**, reconstruct the host-observed source, and validate its native signature. Do not treat a self-claimed endpoint, matching hex, mDNS advertisement, certificate label or Zod schema success as proof of possession. The host must match the source to the actual connected socket and burn failed/expired challenges.

Device ID is lowercase SHA-256 over `ASCII("ZOOMOUSE-DEVICE-ID-ED25519-V1") + 0x00 + phonePublicKey[32]`. `buildDeviceIdPreimage` only produces bytes for trusted native SHA-256, not a cryptographic identity assertion. Independent synthetic 0x22×32 fixture hash: `17351a36a3b5daff2a95a0eced62dcf89efa1442740bc58c689b637f8a2031c5`; Rust-run signature/hash fixture confirmation is a **pending cross-language gate**.

## QR host-key pin

Wire format matching Rust discovery: `zoomouse://pair/v1?ip=<private IP>&port=<1..65535>&host_key=<64 lowercase hex>`. It includes **no bearer token, grant, session, permission or signature**. Only exact unique keys accepted. IPv4 is RFC1918 with Rust's last-octet 0/255 exclusion, IPv6 must be FC00::/7 without zones or embedded IPv4; reject WAN/CGNAT/loopback/multicast/link-local, percent escapes, duplicates, fragments or overly long QR. The QR is a trust anchor **only if independently scanned/verified locally**; mDNS metadata is untrusted.

## Runtime and acceptance boundaries

Rust owns CSPRNG, nonzero phone nonce, Ed25519 key-point validation, 60-second challenge TTL, max 16 concurrent challenges, one-time proof consumption, source and user approval, revocation, and up to 10-minute enrollment. Enrollment alone is not an authenticated DTLS input session. Actual transport/session/Windows desktop binding, firewall LAN confinement and trusted consent-gated M1-E input remain unimplemented. `session-policy` cannot be constructed from this untrusted JSON. Physical M1 cases A01–A17 remain NOT RUN; native Ed25519 cross-language signature and real Android↔Windows integration remain pending.
