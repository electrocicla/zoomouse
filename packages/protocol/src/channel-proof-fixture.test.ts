import { describe, expect, it } from "vitest";
import {
  buildChannelProofPreimage, channelProofFixtureSchema,
  OneShotChannelProofFixtureInspection, parseChannelProofFixture,
  serializeChannelProofFixture, CHANNEL_PROOF_FIXTURE
} from "./channel-proof-fixture";
import type { ChannelProofObservationFixture } from "./channel-proof-fixture";

const fixture = {
  version: 1,
  type: "channel-proof.test-fixture",
  hostPairingKeyHex: "01".repeat(32),
  enrolledPhoneKeyHex: "02".repeat(32),
  localDtlsCertificateSha256Hex: "03".repeat(32),
  peerDtlsCertificateSha256Hex: "04".repeat(32),
  dtlsExporterHex: "05".repeat(32),
  sctpStreamId: 17,
  hostNonceHex: "06".repeat(32),
  signatureHex: "07".repeat(64)
} as const;

const toHex = (bytes: Uint8Array): string => Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");

function preimage(fields: ChannelProofObservationFixture = fixture): Uint8Array {
  return buildChannelProofPreimage({
    hostPairingKeyHex: fields.hostPairingKeyHex,
    enrolledPhoneKeyHex: fields.enrolledPhoneKeyHex,
    localDtlsCertificateSha256Hex: fields.localDtlsCertificateSha256Hex,
    peerDtlsCertificateSha256Hex: fields.peerDtlsCertificateSha256Hex,
    dtlsExporterHex: fields.dtlsExporterHex,
    sctpStreamId: fields.sctpStreamId,
    hostNonceHex: fields.hostNonceHex
  });
}

describe("Rust R7 channel-binding preimage fixture (NOT native authorization)", () => {
  it("matches domain, field ordering, big-endian stream and external SHA-256 vector", async () => {
    const bytes = preimage();
    const hex = toHex(bytes);
    expect(bytes).toHaveLength(245);
    expect(hex).toBe(
      toHex(Uint8Array.from("ZOOMOUSE-DTLS-SCTP-CHANNEL-PROOF-ED25519-SHA256-V1\0", char => char.charCodeAt(0))) +
      "01".repeat(32) + "02".repeat(32) + "03".repeat(32) +
      "04".repeat(32) + "05".repeat(32) + "0011" + "06".repeat(32)
    );
    const cryptoApi = globalThis as unknown as { crypto?: { subtle?: { digest(algorithm: "SHA-256", data: ArrayBuffer): Promise<ArrayBuffer> } } };
    if (!cryptoApi.crypto?.subtle) throw new Error("SHA-256 test runtime unavailable");
    const digest = await cryptoApi.crypto.subtle.digest("SHA-256", Uint8Array.from(bytes).buffer);
    expect(toHex(new Uint8Array(digest))).toBe(
      "ed8e3b2a9850e4cd0d462b326a5b6f104d37a7b10d8e247832a866f44d645ea0"
    );
    expect(CHANNEL_PROOF_FIXTURE.ttlMs).toBe(20_000);
  });

  it("binds every native observation, nonce, key and stream field", () => {
    const original = toHex(preimage());
    const variations = [
      { ...fixture, hostPairingKeyHex: "08".repeat(32) },
      { ...fixture, enrolledPhoneKeyHex: "08".repeat(32) },
      { ...fixture, localDtlsCertificateSha256Hex: "08".repeat(32) },
      { ...fixture, peerDtlsCertificateSha256Hex: "08".repeat(32) },
      { ...fixture, dtlsExporterHex: "08".repeat(32) },
      { ...fixture, sctpStreamId: 18 },
      { ...fixture, hostNonceHex: "08".repeat(32) }
    ];
    for (const altered of variations) {
      expect(toHex(preimage(altered))).not.toBe(original);
    }
    expect(toHex(preimage({ ...fixture, sctpStreamId: 65535 })).slice(-68, -64)).toBe("ffff");
  });

  it("canonical test-fixture roundtrip NEVER claims crypto or a native grant", () => {
    const raw = serializeChannelProofFixture(fixture);
    expect(parseChannelProofFixture(raw)).toEqual({
      ok: true, fixture, nativeAuthority: false
    });
    expect(channelProofFixtureSchema.safeParse(fixture).success).toBe(true);
    expect(() => serializeChannelProofFixture({ ...fixture, authorized: true })).toThrow();
  });

  it("rejects unknown/malformed/oversized/duplicate/noncanonical fixture JSON", () => {
    const raw = serializeChannelProofFixture(fixture);
    const broken = [
      JSON.stringify({ ...fixture, version: 2 }),
      JSON.stringify({ ...fixture, type: "pairing.host-challenge" }),
      JSON.stringify({ ...fixture, command: "powershell" }),
      JSON.stringify({ ...fixture, signatureHex: "ff".repeat(63) }),
      JSON.stringify({ ...fixture, signatureHex: "00".repeat(64) }),
      JSON.stringify({ ...fixture, hostNonceHex: "00".repeat(32) }),
      JSON.stringify({ ...fixture, hostPairingKeyHex: "FF".repeat(32) }),
      JSON.stringify({ ...fixture, dtlsExporterHex: "00".repeat(32) }),
      JSON.stringify({ ...fixture, sctpStreamId: -1 }),
      JSON.stringify({ ...fixture, sctpStreamId: 65536 }),
      JSON.stringify({ ...fixture, sctpStreamId: 17.25 }),
      JSON.stringify({ ...fixture, sctpStreamId: "17" }),
      raw.replace('"version":1', '"version":1,"version":1'),
      " " + raw,
      raw.replace('"version":1', '"version":1 '),
      raw.replace('"version":1', '"\\u0076ersion":1'),
      raw.replace('"version":1', '"vérsion":1')
    ];
    for (const bad of broken) expect(parseChannelProofFixture(bad).ok).toBe(false);
    expect(parseChannelProofFixture("{")).toEqual({ ok: false, reason: "invalid_json" });
    expect(parseChannelProofFixture(undefined)).toEqual({ ok: false, reason: "invalid_frame" });
    expect(parseChannelProofFixture("X".repeat(1025))).toEqual({ ok: false, reason: "frame_too_large" });
    expect(parseChannelProofFixture("🐈".repeat(300)).ok).toBe(false);
  });

  it("burns the attempt on success OR malformed input, never yielding native authority", () => {
    const raw = serializeChannelProofFixture(fixture);
    const guard = new OneShotChannelProofFixtureInspection();
    const first = guard.inspect(raw);
    expect(first.inspected).toBe(true);
    if (first.inspected) {
      expect(first.nativeAuthority).toBe(false);
      expect(first.preimage).toEqual(preimage());
      expect(first.unverifiedSignature).toHaveLength(64);
    }
    expect(guard.inspect(raw)).toEqual({ inspected: false, reason: "replayed" });
    const malformed = new OneShotChannelProofFixtureInspection();
    expect(malformed.inspect("{")).toEqual({ inspected: false, reason: "invalid_json" });
    expect(malformed.inspect(raw)).toEqual({ inspected: false, reason: "replayed" });
  });
});
