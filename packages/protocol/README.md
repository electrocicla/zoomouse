# Zoomouse M1-A: LAN session protocol policy (v1 foundation)

Status: **policy/contracts only**. This package does not implement LAN sockets,
mDNS, QR trust, authenticated key exchange, WebRTC DataChannels, Windows input
injection, capture, or native user consent. M0 preview stays simulated.

## Integration surface (additive, stable existing v1 packet schemas)

Import M1-A names from `@zoomouse/protocol/session-policy` (dedicated export; no cyclic re-export through the existing `@zoomouse/protocol` v1 entrypoint):

- `pairingIntentSchema`: strict **untrusted wire** device introduction
  `{ version: 1, type: "pairing.intent", requestId, deviceId, displayName, clientNonce }`.
  Advertised identifiers and clientNonce are claims, not authenticated identity.
- `sessionNoticeSchema`: strict **informational wire** reply
  `{ version: 1, type: "session.notice", requestId, deviceId, sessionId, expiresAtMs, consentGeneration }`.
  Clients never use this receipt as authorization.
- `localConsentSchema` and `LocalConsent`: strictly validated **trusted host
  record**, explicitly not accepted from remote packets. Contains consentId,
  requestId, deviceId, sessionId, transportBindingId, windowsUserSessionId,
  permission (`remote-input | view-only | denied`), generation and bounded
  issue/expiry timestamps. Its generation and host clock are
  native-trusted authority only; session.notice generation is untrusted data.
- `TrustedSessionContext`: native host-authored snapshot, including authenticated
  device ID, transport channel binding, interactive Windows user session,
  connection/revocation state, `currentConsentGeneration` from the live trusted
  grant authority and the locally issued consent. The input gate rejects a
  mismatched or invalid host generation, even if a stale consent record remains.
  Must only be built from native identity/consent authorities; TS structural
  typing does not enforce provenance.
- `initialInputSessionState(): InputSessionState`: NEW state per **fresh,
  authenticated** session/transport lifecycle.
- `parseInputFrame(raw: unknown)`: type-safe strict v1 input JSON and 4 KiB UTF-8 frame
  check, no authorization side effects.
- `authorizeInputFrame(raw, trustedContext, currentState, trustedHostNowMs):
  InputDecision`: immutable, pure admission gate. A successful result contains
  the parsed `packet`, proposed `next` state and `release` directive when the
  wire input is `input.releaseAll`. Rejection contains explicit reason and
  unchanged state. No Windows action occurs in this package.
- `releaseHeldInputs(state, reason)`: derives a bounded held-key/button
  release directive and a **terminal (non-reusable)** state, even when no network packet may be
  authorized. Reasons: disconnect, reconnect, revoked, expired, desktop-locked,
  dispatch-failed.

## Explicit M1-D/M1-E handshake decisions and trust boundaries

1. Discovery (mDNS, direct IP or QR) obtains **untrusted** host metadata. The
   client sends `pairing.intent`; input packets are forbidden at this stage.
2. **M1-D must implement authenticated, replay-resistant key agreement and bind**
   the authenticated device ID to the transport session plus its unique
   `transportBindingId`. A bare deviceId, nonce, certificate fingerprint,
   DataChannel name, TLS connectivity or JSON notice is not authorization.
   Strong CSPRNG IDs, peer certificate pinning/key confirmation, transcript
   binding, revocation persistence and token/key rotation remain M1-D work.
3. The local interactive Windows user sees identity, requested capability and
   limited expiry; trusted UI records `LocalConsent` for the exact device,
   session, authenticated binding and Windows user session. Default is DENY;
   no privilege or input/capture scope is inferred from viewing consent.
   `remote-input` lease must be positive and <= 10 minutes. The trusted host
   must advance the current consent generation on revoke, expiry or re-approval,
   invalidate previous approvals, and never reconstruct a prior generation from
   remote data. Every reconnect creates a **fresh sessionId/binding and consent**;
   notices are not grants.
4. Only **M1-E** creates `TrustedSessionContext` from authenticated native
   authorities and passes each UTF-8 JSON frame with its own session state into
   `authorizeInputFrame`. The host supplies time from a monotonic-clock
   anchored wall-time source; if host time goes backwards the gate fails closed.
   The successful proposed state is committed **only after reliable native
   dispatch**; dispatch failure must revoke, release and close the session.
5. Until channel-specific sequence arbitration is explicitly designed, **all
   control messages share one ordered reliable stream**. The current single
   strictly increasing sequence rejects reordered input to prevent a late
   key-down after key-up; an unordered/lossy move DataChannel must not share
   the same sequence without separate authenticated per-channel state and
   ordering semantics. This is a deliberate departure from the future
   dual-channel optimization in ADR-0001 and needs parent integration review.
6. On revocation, disconnect, expiry, desktop lock, failure or reconnect, the
   native executor must issue every returned held-key/button release in its
   interactive user session, invalidate the session and destroy its gate state.
   The returned terminal state rejects all future input even if the old context
   mistakenly remains connected. NEVER call initialInputSessionState again for
   that old sessionId/transportBindingId; a fresh native-authenticated session,
   new binding, consent generation and user approval are prerequisites.
   Never rely on an unauthenticated client to send `input.releaseAll`. Release
   must be idempotent at the OS boundary; this library only describes it.

## Enforced policy for accepted input frames

- Wire: exact protocol version 1; Zod discriminated union with no unknown
  properties; raw UTF-8 JSON maximum **4096 bytes**. Schema validation alone is
  not an authenticated transport.
- Time: input no more than **5000 ms** old or **2000 ms** into the future;
  local consent expires strictly, lease max **600000 ms**, interactive desktop
  required. Expiry/revocation is checked against host authority **per packet**.
- Sequence: starts at **1**, strictly increasing; maximum forward gap **64**.
  A **64-bit recent replay bitmap** classifies duplicate and out-of-order
  packets, but never authorizes reordered input; no integer wrap.
- Rates (1-second fixed host window, accepted events only): total <= **90**,
  pointer moves <= **60**, text messages <= **5**. Outside the policy: DoS,
  network pre-auth packet throttling, bounded queues and coalescing belong to
  M1-D/E. Rejected packets never advance replay/rate/held state.
- Held state: at most **32** keyboard codes and 3 pointer buttons; reject
  duplicate downs, orphan ups and unsupported keyboard codes. Native
  dispatcher must separately validate Windows key-code mappings and reject
  privileged/system/secure-desktop operations. Keyboard modifiers are existing
  v1 intent metadata, not OS key hold authorization.
- `keyboard.text` is not a clipboard grant, shell command, IME proof, or
  permission to send secrets; native host must restrict active desktop
  context and protect sensitive fields. `releaseAll` releases tracked state
  but must not clear host revocation/consent rules.

### Security guarantees intentionally NOT claimed

No cryptographic pairing, secure channel, identity verification, signed
consent, trusted clock implementation, memory/process isolation, OS input
injection, Windows session containment, key-up completion receipt, network
anti-DoS, firewall scope, or end-to-end tests are supplied here. Native Rust
implementation must provide wire-equivalent parsers, test vectors, gate
ordering, CSPRNG IDs and OS cleanup; the TypeScript gate alone is insufficient
for release readiness.

The existing `inputPacketSchema`, `peerAdvertisementSchema`,
`zoomSettingsSchema` and `PROTOCOL_VERSION = 1` are unchanged. This seam adds
strict optional integration entry points without breaking the demo's
`ZoomSettings` import.
