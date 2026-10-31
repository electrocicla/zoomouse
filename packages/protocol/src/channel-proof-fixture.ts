import { z } from "zod";
import { PROTOCOL_VERSION } from "./index";

/**
 * Source-only Rust R7 transcript FIXTURE foundation, NOT a production wire
 * format or verifier. Rust currently exposes native structs, not a JSON
 * channel-proof frame. Untrusted values cannot establish DTLS provenance.
 */
export const CHANNEL_PROOF_FIXTURE = Object.freeze({
  version: PROTOCOL_VERSION,
  maxJsonBytes: 1024,
  ttlMs: 20_000,
  domain: "ZOOMOUSE-DTLS-SCTP-CHANNEL-PROOF-ED25519-SHA256-V1\0"
});

const nonzeroHex32 = z.string().regex(/^[0-9a-f]{64}$/).refine(value => !/^0+$/.test(value));
const nonzeroHex64 = z.string().regex(/^[0-9a-f]{128}$/).refine(value => !/^0+$/.test(value));

/**
 * Same native field order as crates/host/src/channel_binding.rs.
 * Fields must come from one native established DTLS/SCTP connection.
 * This TypeScript structural check CANNOT establish that provenance.
 */
export const channelProofObservationFixtureSchema = z.strictObject({
  hostPairingKeyHex: nonzeroHex32,
  enrolledPhoneKeyHex: nonzeroHex32,
  localDtlsCertificateSha256Hex: nonzeroHex32,
  peerDtlsCertificateSha256Hex: nonzeroHex32,
  dtlsExporterHex: nonzeroHex32,
  sctpStreamId: z.number().int().min(0).max(65535),
  hostNonceHex: nonzeroHex32
});

export const channelProofFixtureSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("channel-proof.test-fixture"),
  ...channelProofObservationFixtureSchema.shape,
  signatureHex: nonzeroHex64
});

export type ChannelProofFixture = z.infer<typeof channelProofFixtureSchema>;
export type ChannelProofObservationFixture = z.infer<typeof channelProofObservationFixtureSchema>;
export type ParseChannelProofFixtureResult =
  | Readonly<{ ok: true; fixture: ChannelProofFixture; nativeAuthority: false }>
  | Readonly<{ ok: false; reason: "invalid_frame" | "invalid_json" | "frame_too_large" }>;

/**
 * Accept ONLY the canonical test-fixture encoding. Besides rejecting extra
 * fields, this rejects duplicate JSON keys and ambiguous escape/ordering
 * variants that ordinary JSON.parse would otherwise silently normalize.
 * It is NOT a native wire codec, even when parsing succeeds.
 */
export function parseChannelProofFixture(raw: unknown): ParseChannelProofFixtureResult {
  if (typeof raw !== "string") return { ok: false, reason: "invalid_frame" };
  if (raw.length > CHANNEL_PROOF_FIXTURE.maxJsonBytes) {
    return { ok: false, reason: "frame_too_large" };
  }
  if (!/^[\x20-\x7e]*$/.test(raw)) return { ok: false, reason: "invalid_frame" };
  let decoded: unknown;
  try {
    decoded = JSON.parse(raw) as unknown;
  } catch {
    return { ok: false, reason: "invalid_json" };
  }
  const parsed = channelProofFixtureSchema.safeParse(decoded);
  if (!parsed.success || JSON.stringify(parsed.data) !== raw) {
    return { ok: false, reason: "invalid_frame" };
  }
  return { ok: true, fixture: parsed.data, nativeAuthority: false };
}

export function serializeChannelProofFixture(value: unknown): string {
  const fixture = channelProofFixtureSchema.parse(value);
  const wire = JSON.stringify(fixture);
  if (wire.length > CHANNEL_PROOF_FIXTURE.maxJsonBytes) {
    throw new RangeError("Channel-proof fixture exceeds maximum size");
  }
  return wire;
}

function hexBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

/**
 * Byte-exact preimage for Rust SHA-256 transcript_digest(), BEFORE SHA-256:
 * domain+NUL | hostKey32 | phoneKey32 | localCert32 | peerCert32 |
 * exporter32 | sctpStreamId u16 BE | nonce32 = 245 bytes.
 * Rust signs/verifies SHA256(preimage), NOT preimage itself.
 * Producing this byte array is NEVER a signature check or host approval.
 */
export function buildChannelProofPreimage(value: ChannelProofObservationFixture): Uint8Array {
  const fields = channelProofObservationFixtureSchema.parse(value);
  const domain = Uint8Array.from(
    CHANNEL_PROOF_FIXTURE.domain,
    character => character.charCodeAt(0)
  );
  const parts: readonly Uint8Array[] = [
    domain,
    hexBytes(fields.hostPairingKeyHex),
    hexBytes(fields.enrolledPhoneKeyHex),
    hexBytes(fields.localDtlsCertificateSha256Hex),
    hexBytes(fields.peerDtlsCertificateSha256Hex),
    hexBytes(fields.dtlsExporterHex),
    Uint8Array.of(fields.sctpStreamId >>> 8, fields.sctpStreamId & 255),
    hexBytes(fields.hostNonceHex)
  ];
  const result = new Uint8Array(245);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  if (offset !== result.length) throw new Error("Channel-proof preimage length mismatch");
  return result;
}

export type ChannelProofFixtureInspection =
  | Readonly<{
    inspected: true;
    nativeAuthority: false;
    preimage: Uint8Array;
    unverifiedSignature: Uint8Array;
  }>
  | Readonly<{
    inspected: false;
    reason: "replayed" | "invalid_frame" | "invalid_json" | "frame_too_large";
  }>;

/**
 * Test-only single-use inspection, burning even malformed first attempts
 * like Rust finish(). NOT a replay database, clock, signature verifier,
 * DTLS observation, approval, or permission to send input.
 */
export class OneShotChannelProofFixtureInspection {
  private consumed = false;

  inspect(raw: unknown): ChannelProofFixtureInspection {
    if (this.consumed) return { inspected: false, reason: "replayed" };
    this.consumed = true;
    const result = parseChannelProofFixture(raw);
    if (!result.ok) return { inspected: false, reason: result.reason };
    const fixture = result.fixture;
    const fields: ChannelProofObservationFixture = {
      hostPairingKeyHex: fixture.hostPairingKeyHex,
      enrolledPhoneKeyHex: fixture.enrolledPhoneKeyHex,
      localDtlsCertificateSha256Hex: fixture.localDtlsCertificateSha256Hex,
      peerDtlsCertificateSha256Hex: fixture.peerDtlsCertificateSha256Hex,
      dtlsExporterHex: fixture.dtlsExporterHex,
      sctpStreamId: fixture.sctpStreamId,
      hostNonceHex: fixture.hostNonceHex
    };
    return {
      inspected: true,
      nativeAuthority: false,
      preimage: buildChannelProofPreimage(fields),
      unverifiedSignature: hexBytes(fixture.signatureHex)
    };
  }
}
