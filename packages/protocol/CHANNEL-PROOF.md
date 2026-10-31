# R8 — cross-platform channel-proof transcript: test-only foundation

**Status: SOURCE-ONLY FIXTURE; NOT A PRODUCTION WIRE PROTOCOL OR AUTHENTICATED CHANNEL.** This is additive to `@zoomouse/protocol` under `@zoomouse/protocol/channel-proof-fixture`. Android and Windows native code do **not** import this fixture or obtain new permissions from it.

## Grounded Rust R7 contract

Rust `crates/host/src/channel_binding.rs` (R7 `d57900acb909`) implements a one-shot Ed25519 proof over a native-observed DTLS/SCTP association. Its `transcript_digest` signs and strictly verifies **SHA-256** over this exact **245-byte preimage**:

```text
ASCII("ZOOMOUSE-DTLS-SCTP-CHANNEL-PROOF-ED25519-SHA256-V1") || 0x00  (51)
host_pairing_key                              (32)
enrolled_phone_key                            (32)
local_dtls_certificate_sha256                (32)
peer_dtls_certificate_sha256                 (32)
dtls_exporter                                 (32)
sctp_stream_id, unsigned big-endian u16        (2)
host_nonce, OS-generated nonzero 32 bytes     (32)
TOTAL                                        (245)
```

Native R7 `start()` checks nonzero keys/fingerprints/exporter and validates the enrolled Ed25519 phone public key, generates a fresh host nonce with OS CSPRNG, stores the SHA-256 digest, and expects the signature from that phone key. Native `finish()` consumes the challenge **before** checking the signature, deadline, or clock regression; TTL is strictly below 20 seconds. A matching signature still **does not create verified native DTLS evidence**, host consent or an input grant.

## Scope, parser and negative vectors

The `channel-proof.test-fixture` JSON shape in `channel-proof-fixture.ts` is a **new local test-vector representation**, not a Rust or WebRTC wire contract. The native Rust module presently has no JSON/frame encoder, renderer API, Android counterpart, or trusted producer for its `ObservedEncryptedChannel`. No API in this module can authenticate an observation, establish key ownership, verify Ed25519, authorize a session, issue permission, send input, or supply DTLS fingerprints/exporter. Its `nativeAuthority` property is the literal `false`.

Test fixtures use protocol version 1, lowercase hex strings for 32-byte native fields and 64-byte unverified signatures, a numeric u16 stream ID, no unknown keys, and a conservative ASCII canonical JSON budget <= 1024 bytes. Canonical encoding refuses duplicate keys, key reordering, noncanonical escapes and arbitrary whitespace. Non-ASCII, zero-filled key/nonce/signature and oversized values fail closed. The one-shot inspection object burns its first attempt even on malformed input; this is **in-memory fixture hygiene**, not host anti-replay protection. Do not use its result as the production native verifier or a wire message.

Pinned synthetic preimage vector: host key `01` x32, phone key `02` x32, local cert `03` x32, peer cert `04` x32, exporter `05` x32, stream ID `17` (wire bytes `00 11`), nonce `06` x32. The independent SHA-256 preimage digest is:

```text
ed8e3b2a9850e4cd0d462b326a5b6f104d37a7b10d8e247832a866f44d645ea0
```

This vector checks ordering and hashing **only**. Synthetic key bytes are not evidence of a valid Ed25519 public point or signature. Separate Rust tests already exercise wrong exporter, host/peer certificate, key, stream, nonce, wrong phone, one-time consumption, expired and regressed clocks. New TS negative vectors check the preimage changes for every field, strict shape and replayed fixture inspections.

## Missing production integration — fail closed

The **missing native dependency** is a reviewed WebRTC/DTLS/SCTP adapter that obtains an authenticated established association's exporter, both directionally correct certificate SHA-256 fingerprints, stream identifier and enrollment-bound keys from the same native connection; an Android counterpart must obtain matching exporter/host identity and execute real Ed25519 signing. The current Rust R7 module has **no serialized challenge/response contract** suitable for TS↔Rust interop, no native exporter producer, and no host-owned local OS consent provenance wired to this flow. A production wire schema and reliable handshake/replay lifecycle must be jointly designed with native host and Android owners and tested with Rust↔Android vectors before adding a production codec or `verify` API.

**Absolutely no** renderer/JS/network/QR/fixture field may be promoted into `ObservedEncryptedChannel`, `VerifiedChannelEvidence`, `TrustedSessionContext`, `HostApprovedSession` or OS input. Production must keep `DisabledMissingNativeAuthority`. Physical M1 acceptance **A01–A17 remain NOT RUN**; this fixture does not satisfy M1, M2, packaging or E2E release gates.
