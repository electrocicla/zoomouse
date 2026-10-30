import { describe, expect, it } from "vitest";
import {
  buildDeviceIdPreimage, buildPairingTranscript, hostChallengeSchema, hostQrPinSchema,
  parseHostQrPin, parsePairingFrame, phoneHelloSchema, phoneProofSchema,
  privateIpOctets, privateLanEndpointSchema, serializeHostQrPin, serializePairingFrame
} from "./pairing-wire";

const hex = (bytes: Uint8Array): string => Array.from(bytes, value => value.toString(16).padStart(2, "0")).join("");
const hello = {
  version: 1, type: "pairing.phone-hello", phonePublicKeyHex: "22".repeat(32),
  phoneNonceHex: "55".repeat(32), displayName: "Phone X"
} as const;
const challenge = {
  version: 1, type: "pairing.host-challenge", requestIdHex: "33".repeat(16),
  hostPublicKeyHex: "11".repeat(32), hostNonceHex: "44".repeat(32),
  hostSignatureHex: "66".repeat(64), deviceIdHex: "aa".repeat(32),
  observedPhoneEndpoint: { address: "192.168.1.5", port: 50001 }
} as const;
const proof = {
  version: 1, type: "pairing.phone-proof",
  requestIdHex: challenge.requestIdHex, phoneSignatureHex: "77".repeat(64)
} as const;

describe("pairing wire v1 (untrusted shape only)", () => {
  it("roundtrips three versioned JSON packets with no added capabilities", () => {
    for (const packet of [hello, challenge, proof]) {
      expect(parsePairingFrame(serializePairingFrame(packet))).toEqual({ ok: true, frame: packet });
    }
    expect(phoneHelloSchema.safeParse(hello).success).toBe(true);
    expect(hostChallengeSchema.safeParse(challenge).success).toBe(true);
    expect(phoneProofSchema.safeParse(proof).success).toBe(true);
  });

  it("rejects versions, extra keys, malformed lengths, Unicode byte overrun and zero nonce", () => {
    const malformed = [
      { ...hello, version: 2 }, { ...hello, command: "run" },
      { ...hello, phoneNonceHex: "00".repeat(32) },
      { ...hello, phonePublicKeyHex: "FF".repeat(32) },
      { ...hello, phonePublicKeyHex: "aa".repeat(31) },
      { ...hello, displayName: "🐈".repeat(33) },
      { ...hello, displayName: "\ud800" }, { ...hello, displayName: "   " },
      { ...challenge, requestIdHex: "ff".repeat(15) },
      { ...challenge, hostSignatureHex: "ff".repeat(63) },
      { ...challenge, observedPhoneEndpoint: { address: "8.8.8.8", port: 50001 } },
      { ...challenge, observedPhoneEndpoint: { address: "192.168.1.5", port: 0 } },
      { ...proof, phoneSignatureHex: "ff".repeat(65) }, { ...proof, grant: "approved" }
    ];
    for (const packet of malformed) {
      expect(parsePairingFrame(JSON.stringify(packet))).toEqual({ ok: false, reason: "invalid_frame" });
    }
    expect(parsePairingFrame("{")).toEqual({ ok: false, reason: "invalid_json" });
    expect(parsePairingFrame(undefined)).toEqual({ ok: false, reason: "invalid_frame" });
    expect(parsePairingFrame("x".repeat(2049))).toEqual({ ok: false, reason: "frame_too_large" });
    expect(() => serializePairingFrame({ ...hello, inputAllowed: true })).toThrow();
  });
});

describe("byte-exact Rust pairing transcript (test fixture; Rust execution independent)", () => {
  const fields = {
    hostPublicKeyHex: challenge.hostPublicKeyHex,
    phonePublicKeyHex: hello.phonePublicKeyHex,
    requestIdHex: challenge.requestIdHex,
    hostNonceHex: challenge.hostNonceHex,
    phoneNonceHex: hello.phoneNonceHex,
    observedPhoneEndpoint: challenge.observedPhoneEndpoint
  };
  it("rejects WAN/loopback/link-local/multicast/CGNAT and invalid literals", () => {
    for (const ip of [
      "8.8.8.8", "0.0.0.0", "127.0.0.1", "169.254.10.1", "100.64.1.1",
      "192.168.1.0", "192.168.1.255", "10.0.0.255", "192.168.001.5",
      "::", "::1", "fe80::1", "ff02::fb", "2001:db8::1",
      "::ffff:192.168.1.1", "fd12::1%eth0", "fd12:::2"
    ]) expect(privateIpOctets(ip)).toBeNull();
    for (const ip of ["10.1.2.3", "172.16.3.4", "172.31.9.44", "192.168.1.5", "fd12::2", "fc00:1234::5"])
      expect(privateIpOctets(ip)).not.toBeNull();
    expect(privateLanEndpointSchema.safeParse({ address: "fd12::2", port: 65536 }).success).toBe(false);
  });

  it("matches the exact 180-byte v4 preimage, including family tag and BE port", () => {
    const result = buildPairingTranscript(fields);
    expect(result.length).toBe(180);
    expect(hex(result)).toBe(
      "5a4f4f4d4f5553452d4c414e2d504149522d454432353531392d563100" +
      "11".repeat(32) + "22".repeat(32) + "33".repeat(16) +
      "44".repeat(32) + "55".repeat(32) + "04c0a80105c351"
    );
  });

  it("binds v6 source family (16 octets) and rejects changed address", () => {
    const result = buildPairingTranscript({
      ...fields, observedPhoneEndpoint: { address: "fd12::2", port: 50001 }
    });
    expect(result.length).toBe(192);
    expect(hex(result).endsWith("06fd120000000000000000000000000002c351")).toBe(true);
    expect(() => buildPairingTranscript({
      ...fields, observedPhoneEndpoint: { address: "2001:db8::2", port: 50001 }
    })).toThrow();
  });

  it("pins SHA-256 device-ID domain preimage for native hash verification", () => {
    expect(hex(buildDeviceIdPreimage(hello.phonePublicKeyHex))).toBe(
      "5a4f4f4d4f5553452d4445564943452d49442d454432353531392d563100" + "22".repeat(32)
    );
    expect(() => buildDeviceIdPreimage("zz".repeat(32))).toThrow();
  });
});

describe("Rust pairing.rs deterministic Ed25519 fixture", () => {
  it("reconstructs the exact host+phone signed 180-byte transcript", () => {
    const hostKey = "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c";
    const phoneKey = "fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618";
    const fields = {
      hostPublicKeyHex: hostKey, phonePublicKeyHex: phoneKey,
      requestIdHex: "11".repeat(16), hostNonceHex: "22".repeat(32),
      phoneNonceHex: "33".repeat(32),
      observedPhoneEndpoint: { address: "192.168.1.5", port: 50001 }
    };
    const expected = "5a4f4f4d4f5553452d4c414e2d504149522d454432353531392d563100" +
      hostKey + phoneKey + "11".repeat(16) + "22".repeat(32) +
      "33".repeat(32) + "04c0a80105c351";
    expect(hex(buildPairingTranscript(fields))).toBe(expected);
    expect(hex(buildDeviceIdPreimage(phoneKey))).toBe(
      "5a4f4f4d4f5553452d4445564943452d49442d454432353531392d563100" + phoneKey
    );
    // Native Rust separately checks digest 082c7c7a40c047564e823982765deaa84f1bd4889149783f27554fe0f5424190,
    // Ed25519 host/phone signatures, and rejection when source/nonce changes.
    expect(hex(buildPairingTranscript({
      ...fields, observedPhoneEndpoint: { address: "192.168.1.6", port: 50001 }
    }))).not.toBe(expected);
  });
});

describe("QR host key pin: no permission or bearer secret", () => {
  const pin = {
    endpoint: { address: "192.168.1.10", port: 50001 },
    hostPublicKeyHex: "a5".repeat(32)
  } as const;
  it("mirrors Rust discovery QR format", () => {
    const encoded = serializeHostQrPin(pin);
    expect(encoded).toBe("zoomouse://pair/v1?ip=192.168.1.10&port=50001&host_key=" + "a5".repeat(32));
    expect(parseHostQrPin(encoded)).toEqual(pin);
    expect(hostQrPinSchema.safeParse({ ...pin, approved: true }).success).toBe(false);
  });
  it("rejects duplicated keys, extra fields, public IP, malformed keys, fragments and overflow", () => {
    const qr = serializeHostQrPin(pin);
    for (const bad of [
      qr + "&port=50001", qr + "&token=1", qr + "#fragment", qr + "x".repeat(400),
      qr.replace("192.168.1.10", "8.8.8.8"),
      qr.replace("50001", "0"), qr.replace("50001", "65536"),
      qr.replace("host_key=", "host_key=%"), qr.replace("a5", "A5"),
      qr.replace("ip=", "ip=[::1]")
    ]) expect(parseHostQrPin(bad)).toBeNull();
  });
});
