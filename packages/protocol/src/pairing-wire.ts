import { z } from "zod";
import { PROTOCOL_VERSION } from "./index";

/**
 * M1-D pairing wire v1. These schemas validate untrusted JSON shape ONLY.
 * Native Ed25519 verification, CSPRNG, observed source, TTL and OS consent
 * remain exclusively trusted host/mobile responsibilities.
 */
export const PAIRING_WIRE = Object.freeze({
  version: PROTOCOL_VERSION,
  maxJsonBytes: 2048,
  maxQrBytes: 256,
  challengeTtlMs: 60_000,
  maxPendingChallenges: 16
});

const hex16 = z.string().regex(/^[0-9a-f]{32}$/);
const hex32 = z.string().regex(/^[0-9a-f]{64}$/);
const hex64 = z.string().regex(/^[0-9a-f]{128}$/);
const nonzeroHex32 = hex32.refine(value => !/^0+$/.test(value));
const endpointPort = z.number().int().min(1).max(65535);

function utf8Length(input: string): number {
  let length = 0;
  for (let index = 0; index < input.length; index += 1) {
    const code = input.charCodeAt(index);
    if (code < 0x80) length += 1;
    else if (code < 0x800) length += 2;
    else if (code >= 0xd800 && code <= 0xdbff) {
      const next = input.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return Number.POSITIVE_INFINITY;
      length += 4;
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return Number.POSITIVE_INFINITY;
    } else length += 3;
  }
  return length;
}

const displayNameSchema = z.string().refine(
  name => name.trim().length > 0 && utf8Length(name) <= 128
);

/** Convert strictly private IP literals to the identical octets Rust IpAddr::octets() yields. */
export function privateIpOctets(address: string): Uint8Array | null {
  if (address.includes(":")) {
    // RFC4193 IPv6 literals, no brackets, scope IDs or embedded IPv4 syntax.
    if (!/^[0-9a-fA-F:]+$/.test(address)) return null;
    const sides = address.split("::");
    if (sides.length > 2) return null;
    const parseSide = (side: string): number[] | null => {
      if (side === "") return [];
      const words = side.split(":");
      if (words.some(word => !/^[0-9a-fA-F]{1,4}$/.test(word))) return null;
      return words.map(word => Number.parseInt(word, 16));
    };
    const left = parseSide(sides[0] ?? "");
    const right = sides.length === 2 ? parseSide(sides[1] ?? "") : [];
    if (left === null || right === null) return null;
    const used = left.length + right.length;
    if (sides.length === 2 ? used >= 8 : used !== 8) return null;
    const words = [...left, ...Array<number>(8 - used).fill(0), ...right];
    if (words.length !== 8) return null;
    const first = words[0];
    if (first === undefined || (first >>> 8) !== 0xfc && (first >>> 8) !== 0xfd) return null;
    const octets = new Uint8Array(16);
    for (let i = 0; i < 8; i += 1) {
      const value = words[i];
      if (value === undefined) return null;
      octets[2 * i] = value >>> 8;
      octets[2 * i + 1] = value & 255;
    }
    return octets;
  }
  const parts = address.split(".");
  if (parts.length !== 4 || parts.some(part => !/^(0|[1-9][0-9]{0,2})$/.test(part))) return null;
  const values = parts.map(Number);
  const [a, b, , d] = values;
  if (a === undefined || b === undefined || d === undefined ||
      values.some(value => value > 255) || d === 0 || d === 255) return null;
  if (!(a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168))) return null;
  return Uint8Array.from(values);
}

export const privateLanEndpointSchema = z.strictObject({
  address: z.string().min(1).max(45).refine(value => privateIpOctets(value) !== null),
  port: endpointPort
});

export const phoneHelloSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("pairing.phone-hello"),
  phonePublicKeyHex: nonzeroHex32,
  phoneNonceHex: nonzeroHex32,
  displayName: displayNameSchema
});

export const hostChallengeSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("pairing.host-challenge"),
  requestIdHex: hex16,
  hostPublicKeyHex: nonzeroHex32,
  hostNonceHex: hex32,
  hostSignatureHex: hex64,
  deviceIdHex: hex32,
  /** Socket source observed BY THE HOST. Included in the signed Rust transcript. */
  observedPhoneEndpoint: privateLanEndpointSchema
});

export const phoneProofSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("pairing.phone-proof"),
  requestIdHex: hex16,
  phoneSignatureHex: hex64
});

export const pairingWireFrameSchema = z.discriminatedUnion("type", [
  phoneHelloSchema, hostChallengeSchema, phoneProofSchema
]);

export const hostQrPinSchema = z.strictObject({
  endpoint: privateLanEndpointSchema,
  hostPublicKeyHex: nonzeroHex32
});

export type PhoneHelloWire = z.infer<typeof phoneHelloSchema>;
export type HostChallengeWire = z.infer<typeof hostChallengeSchema>;
export type PhoneProofWire = z.infer<typeof phoneProofSchema>;
export type PairingWireFrame = z.infer<typeof pairingWireFrameSchema>;
export type PrivateLanEndpointWire = z.infer<typeof privateLanEndpointSchema>;
export type HostQrPin = z.infer<typeof hostQrPinSchema>;

export type ParsePairingFrameResult =
  | Readonly<{ ok: true; frame: PairingWireFrame }>
  | Readonly<{ ok: false; reason: "invalid_frame" | "invalid_json" | "frame_too_large" }>;

export function parsePairingFrame(raw: unknown): ParsePairingFrameResult {
  if (typeof raw !== "string") return { ok: false, reason: "invalid_frame" };
  if (utf8Length(raw) > PAIRING_WIRE.maxJsonBytes) return { ok: false, reason: "frame_too_large" };
  let value: unknown;
  try { value = JSON.parse(raw) as unknown; }
  catch { return { ok: false, reason: "invalid_json" }; }
  const parsed = pairingWireFrameSchema.safeParse(value);
  if (!parsed.success) return { ok: false, reason: "invalid_frame" };
  return { ok: true, frame: parsed.data };
}

/** Serialize a schema-checked v1 JSON frame; never interpret a parsed frame as authorization. */
export function serializePairingFrame(frame: unknown): string {
  const value = pairingWireFrameSchema.parse(frame);
  const encoded = JSON.stringify(value);
  if (utf8Length(encoded) > PAIRING_WIRE.maxJsonBytes) {
    throw new RangeError("Pairing frame exceeds maximum UTF-8 wire bytes");
  }
  return encoded;
}

const transcriptSchema = z.strictObject({
  hostPublicKeyHex: hex32,
  phonePublicKeyHex: hex32,
  requestIdHex: hex16,
  hostNonceHex: hex32,
  phoneNonceHex: hex32,
  observedPhoneEndpoint: privateLanEndpointSchema
});
export type PairingTranscript = z.infer<typeof transcriptSchema>;

function bytesFromHex(value: string): Uint8Array {
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

function asciiBytes(value: string): Uint8Array {
  return Uint8Array.from([...value].map(char => char.charCodeAt(0)));
}

function concatBytes(parts: readonly Uint8Array[]): Uint8Array {
  const total = parts.reduce((count, part) => count + part.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}

const transcriptDomain = asciiBytes("ZOOMOUSE-LAN-PAIR-ED25519-V1\0");
const deviceIdDomain = asciiBytes("ZOOMOUSE-DEVICE-ID-ED25519-V1\0");

/**
 * Byte-for-byte Rust pairing::transcript preimage:
 * domain + hostKey32 + phoneKey32 + requestId16 + hostNonce32 + phoneNonce32
 * + address family (4/6) + source IP octets (4/16) + big-endian source port.
 * Both Ed25519 signatures bind exactly these bytes. No signature verification here.
 */
export function buildPairingTranscript(fields: PairingTranscript): Uint8Array {
  const input = transcriptSchema.parse(fields);
  const ip = privateIpOctets(input.observedPhoneEndpoint.address);
  if (ip === null) throw new TypeError("Invalid private LAN source");
  const port = input.observedPhoneEndpoint.port;
  return concatBytes([
    transcriptDomain,
    bytesFromHex(input.hostPublicKeyHex),
    bytesFromHex(input.phonePublicKeyHex),
    bytesFromHex(input.requestIdHex),
    bytesFromHex(input.hostNonceHex),
    bytesFromHex(input.phoneNonceHex),
    Uint8Array.of(ip.length === 4 ? 4 : 6),
    ip,
    Uint8Array.of(port >>> 8, port & 255)
  ]);
}

/** SHA-256 preimage for Rust identity_for(); hashing must occur in trusted native crypto. */
export function buildDeviceIdPreimage(phonePublicKeyHex: string): Uint8Array {
  return concatBytes([deviceIdDomain, bytesFromHex(hex32.parse(phonePublicKeyHex))]);
}

/** QR mirrors Rust DiscoveryDescriptor::qr_payload(): this is a host-key PIN, not an approval. */
export function serializeHostQrPin(input: HostQrPin): string {
  const pin = hostQrPinSchema.parse(input);
  const result = `zoomouse://pair/v1?ip=${pin.endpoint.address}&port=${pin.endpoint.port}&host_key=${pin.hostPublicKeyHex}`;
  if (utf8Length(result) > PAIRING_WIRE.maxQrBytes) throw new RangeError("QR payload too long");
  return result;
}

export function parseHostQrPin(raw: unknown): HostQrPin | null {
  if (typeof raw !== "string" || utf8Length(raw) > PAIRING_WIRE.maxQrBytes) return null;
  const prefix = "zoomouse://pair/v1?";
  if (!raw.startsWith(prefix)) return null;
  const params = raw.slice(prefix.length).split("&");
  if (params.length !== 3) return null;
  const fields = new Map<string, string>();
  for (const param of params) {
    const match = /^(ip|port|host_key)=([^&#?%]+)$/.exec(param);
    if (!match || match[1] === undefined || match[2] === undefined || fields.has(match[1])) return null;
    fields.set(match[1], match[2]);
  }
  const address = fields.get("ip");
  const portString = fields.get("port");
  const hostPublicKeyHex = fields.get("host_key");
  if (address === undefined || portString === undefined || hostPublicKeyHex === undefined ||
      !/^[1-9][0-9]{0,4}$/.test(portString)) return null;
  const result = hostQrPinSchema.safeParse({
    endpoint: { address, port: Number(portString) }, hostPublicKeyHex
  });
  return result.success ? result.data : null;
}
