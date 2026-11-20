# R18-HOST — native Rust TLS1.3 phone-association security seam (2026-11-20)

## Ownership, identity, baseline
- Fresh independently authenticated MK059 task: deployment-child-b66df45e9a95a476505f53e94e2f49ce; instance web-child-b66df45e9a95a476505f53e94e2f49ce; lifecycleEpoch e6b10df4-3ae5-4e27-a526-4e4b1b0fe2be; Zoomouse selected rootIndex13 reverified; canonical github.com/electrocicla/zoomouse branch main, admission HEAD 33507d4223bd (origin/main 0 ahead / 0 behind).
- Parent task deployment-child-0541cc814a8798f2cccd8f9bc57c4546, parent Agents Global session 92db6cba-3603-43ce-9ed7-b916452fd2f1: COORDINATION ONLY. Android worker separately owns apps/mobile/** and its worker report.
- Own exact edits ONLY crates/host/src/tls_association.rs (new), crates/host/src/lib.rs (module registration), this NEW workplan/workers/R18-HOST.md. No changes to previous pairing/DTLS implementation or Tauri production admission.
- Preserve five inherited dirty/untracked paths with no read-modify-write/stage/reset: .gitignore, package.json, pnpm-lock.yaml, workplan/workers/R6-WIN.md, workplan/workers/R13-ANDROID.md. No R17 or parent R18 doc edits.
- Root AGENTS.md and .github/copilot-instructions.md absent (parent verified 404). Read R18 plan/tasks/worklog; R17-HOST; Rust Cargo.toml, native binding, pairing, channel_binding, lib and security context.

## Security implementation and non-negotiable boundary
- crates/host/Cargo.toml currently lists **ed25519-dalek 2.2**, **getrandom 0.2**, **sha2 0.10**, serde 1. Read COMPLETE crates/host/Cargo.lock before modification: exactly **ed25519-dalek 2.2.0**, **getrandom 0.2.17**, **sha2 0.10.9**; no rustls, ring or aws-lc-rs packages present in the locked host dependency graph. No installed/reviewed TLS provider or granted host cert provisioning evidenced. No owner approval to introduce rustls, crypto provider, certificate provision/storage, Cargo.lock modification, exposed TCP listener, firewall change or Windows OS permission.
- Target reviewed native backend: **TLS1.3/TCP**, Rust **rustls 0.23.x** (crypto provider policy and concrete resolved version REQUIRED before install), Android **JSSE SSLSocket TLSv1.3** with independent pinned live X.509 leaf-cert SHA256 or pinned SPKI SHA256 verification and literal RFC1918 IPv4 addresses. Production host certificate/key provisioning policy must define owner-admin approved pinned persistent cert with encrypted OS user-scoped private-key storage, rollover/revocation and QR pin verification policy. No generation of certs/secrets authorized.
- New **crate-private** tls_association.rs is a production-useful *protocol and peer-proof component*, NOT a TLS server/producer. Both VerifiedTlsSocket and LocalEnrollment have **no production constructors**; only #[cfg(test)] synthetic fixtures instantiate them. A future reviewed TLS socket backend must implement constructor *inside the module* only after successful LIVE rustls TLS1.3 handshake, certificate provenance check, pinned fingerprint as observed, kernel socket local_addr/peer_addr and private-interface/route verification, and independent host OS-local enrolled phone-key read. No caller-supplied JSON/hash/QR/insecure TLS can mint these types. No exposed TCP plaintext listener or sharing with test-only pairing_tcp.rs.
- LocalEnrollment revision/active fields must come from a future durable user-scoped **local approved** enrollment/revocation store; no persistence has been introduced, so persistent A2 remains BLOCKED. Certificate observation, phone signature or QR pairing host key never grants Windows input. No VerifiedChannelEvidence, AuthenticatedPeer, HostAdmission or OS approval construction; production stays DisabledInput and closed.
- Process monotonic AtomicU64 counter increments at the start of each association and refuses overflow; epoch is never reused *within one process*. Every attempt draws OS CSPRNG host nonce32, rejects all-zero entropy. Fresh client nonce32 required. **Restart availability/security gate:** same stable TLS certificate + Android persistent high-water epoch makes legitimate post-restart associations reject safely once host epoch resets. This is a real fail-closed availability blocker until owner approves persistent encrypted monotonic epoch scoped to the certificate identity, or a separately authenticated host boot-generation extension and reviewed reconnect/re-enrollment transcript. DO NOT weaken Android replay checks, invent restart evidence, or claim persistent A1/A2 accepted.
- Pending proof is consumed BEFORE parse, signature verification, OS-enrollment revalidation or deadline check; any malformed, stale, bad key, reroute, socket generation change, enrollment revision/revocation, disconnect or clock rollback burns nonce. Strict Ed25519 verification uses independently enrolled key. Successful verification returns only a component-level Ok(()), no security authority.

## Shared deterministic Rust–Android ABI (agreed with R18-ANDROID worker)
The **ONLY** framing is 2-byte big-endian unsigned payload length **1..4096 inclusive** followed by exactly that many bytes (no suffix, no concatenated frames). Fragments may be assembled up to 4098 total bytes including prefix; zero, overflow, trailing bytes, short reads, EOF and expired/incomplete frames are terminal. Each stream transaction has 20-second monotonic PROOF_TTL, no backward time. The future native socket backend must set **socket OS read/write timeouts <=20s** in addition to the pure decoder's monotonic checks; this module never performs blocking I/O.

Host sends **Challenge payload exactly 118 bytes**:
| Byte offsets in payload | Field / size |
| --- | --- |
| [0] | version uint8 = 0x01 |
| [1] | tag uint8 = 0x01 |
| [2..10) | host association epoch uint64BE, >0, process-monotonic |
| [10..42) | observed live host TLS leaf certificate DER SHA-256 bytes32 (NOT pairing key) |
| [42..46) | kernel-observed host/local socket IPv4 bytes4 (network order) |
| [46..48) | kernel-observed host TCP port uint16BE |
| [48..52) | kernel-observed phone/peer socket IPv4 bytes4 |
| [52..54) | kernel-observed phone TCP port uint16BE |
| [54..86) | new OS-CSPRNG host nonce bytes32 |
| [86..118) | independently locally enrolled phone Ed25519 public key bytes32 |

Phone responds with **Response payload exactly 98 bytes**: [0] version=0x01, [1] tag=0x02, [2..34) fresh client nonce32, [34..98) Ed25519 signature64. No caller-supplied host epoch, local ports, certificate or enrollment key field in response; Android independently validates each against actual verified native TLS session before signing.

**Canonical RAW Ed25519 signature preimage, NO intermediate SHA-256 digest**:

    ASCII_BYTES("ZOOMOUSE-TLS13-PHONE-PROOF-V1") || 0x00
    || hostTlsPeerLeafCertSha256[32]
    || hostNonce[32]
    || clientNonce[32]
    || hostAssociationEpoch_u64BE[8]
    || observedHostIPv4[4] || observedHostPort_u16BE[2]
    || observedPhoneIPv4[4] || observedPhonePort_u16BE[2]
    || enrolledPhoneEd25519PublicKey[32]

The signature is **Ed25519.sign(raw preimage)**, verified with strict verification using the host's locally enrolled phone public key. Digest hashes may be computed by Android for diagnostics but MUST NOT replace this raw signed input. The host computes the preimage from its immutable challenge and response nonce. This order was coordinated with R18-ANDROID via Agents Global direct thread da69cf40-a65a-461c-95b3-d768391fda9b. Note that host challenge wire byte ordering differs from the canonical signed preimage; use offsets explicitly, never sign serialized challenge payload blindly.

**Trust gates for future integration:** actual live rustls 0.23.x provider, reviewed installed/locked crypto backend, persistent host certificate private-key scope and rotation, Android JSSE matching leaf pin and provider support, Android Keystore nonexportable Ed25519 availability on real device, independent locally approved persistent enrollment/revocation, exact observed endpoints and private route/IP binding, per-socket TLS generation from host native source, OS-local interactive Windows approval per association, and actual device tests. No handwritten synthetic VerifiedTlsSocket constructor, no test fixture promotion, no plaintext TCP success. A1/A2/A3/A4/A5/A6/A7 remain BLOCKED until independent native/physical evidence.

## Implementation QA and publishing
- Initial cargo fmt --check attempt after source creation was **denied before execution** by Agents Global critical parent coordination. Consumed/acknowledged critical message, replied. Next attempted terminal run was **denied before execution** due disjoint Android worker active file claim; no user-approved bypass or fake exit receipt. Rust tests are not yet a successful validation at the time of this note.
- Source currently includes strict frame/stream hostile fixtures, native opaque no-production-constructor types, SHA-256 leaf fingerprint *carried* (not asserted), one-shot signature, and denial of HostAdmission.
- Scoped Rust validation (governed visible terminal `agent-term-1795196009330-15`): initial `cargo fmt --check` run `termrun_356a3737527a49afadeb3ab6a41e0165` EXIT **1** (formatting-only). Corrected ONLY own `crates/host/src/tls_association.rs` via existing rustfmt `termrun_abd9e12938e14fdbae686f60df8061ad` EXIT **0**.
- `cargo fmt --manifest-path crates/host/Cargo.toml --check`: `termrun_001dffcb63fd417d9e478262f32d4453`, EXIT **0**.
- `cargo check --locked --manifest-path crates/host/Cargo.toml`: `termrun_3beafcfbf2cf4442a6d4016954a10caa`, EXIT **0**.
- `cargo test --locked --manifest-path crates/host/Cargo.toml`: `termrun_e2aa858be7f24a9c8b627b60527c253f`, EXIT **0** — **84 unit tests PASS plus 1 doctest PASS**, eight new host association hostile tests. NO physical socket/TLS handshake tests were run or falsely implied.
- `cargo clippy --locked --manifest-path crates/host/Cargo.toml --all-targets -- -D warnings`: `termrun_9d41c80ef152408c9615471dbf2a9535`, EXIT **0**.
- Failed reservation attempts before command execution: critical Agents Global message pending; Android/parent file claims; later Android's workspace validation turn. Coordinated via inbox/replies/acks, never bypassed; released HOST file claims, gave Android/parent the next workspace QA turn. Cargo.lock remains untouched.
- Exact Git commit/push receipt and 5 inherited dirty paths verification still pending; append only after governed exact-file Git completion. Successful Rust source unit tests **DO NOT** prove TLS live handshake, actual Android association, enrolled credential persistence, Win11 interactive OS approval, EXE/APK or physical A01–A17.
