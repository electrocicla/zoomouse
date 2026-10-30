import { parsePrivateIpv4 } from "./network";
import type { DiscoveredHost } from "./transport";

/** Untrusted candidate information. No candidate proves a peer or permits input. */
export type LanBootstrapCandidate =
  | Readonly<{ kind: "manual-ip"; ipv4: string; port: number; trust: "unverified" }>
  | Readonly<{ kind: "discovered"; deviceId: string; displayName: string; ipv4: string; port: number; trust: "unverified" }>
  | Readonly<{ kind: "qr-host-key-pin"; ipv4: string; port: number; hostPublicKeyHex: string; trust: "key-pin-only" }>;

const QR_PREFIX = "zoomouse://pair/v1?";
const PUBLIC_KEY_HEX = /^[0-9a-f]{64}$/;
const DISCOVERY_ID = /^[a-zA-Z0-9._:-]{1,128}$/;
const VALID_FIELDS = new Set(["ip", "port", "host_key"]);

function validPort(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 65535;
}

function exactPrivateAddress(value: unknown): string | null {
  if (typeof value !== "string") return null;
  // Never normalize a wire-provided address. Reject whitespace and ambiguities.
  const parsed = parsePrivateIpv4(value);
  return parsed === value ? parsed : null;
}

function validDisplayName(name: unknown): name is string {
  if (typeof name !== "string" || name.trim().length === 0) return false;
  // React Native and Node both support Unicode strings, but malformed
  // surrogates and oversized wire labels must not cross the native seam.
  let bytes = 0;
  for (let i = 0; i < name.length; i += 1) {
    const code = name.charCodeAt(i);
    if (code < 0x80) bytes += 1;
    else if (code < 0x800) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff) {
      const next = name.charCodeAt(i + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
      bytes += 4;
      i++;
    } else if (code >= 0xdc00 && code <= 0xdfff) return false;
    else bytes += 3;
    if (bytes > 128) return false;
  }
  return true;
}

/** Explicit user-entered private IPv4 and port; no socket is opened. */
export function manualLanCandidate(input: unknown, port: unknown): LanBootstrapCandidate | null {
  if (typeof input !== "string" || !validPort(port)) return null;
  const ipv4 = parsePrivateIpv4(input);
  return ipv4 === null ? null : { kind: "manual-ip", ipv4, port, trust: "unverified" };
}

/** mDNS output is a discovery hint and cannot be promoted to an identity. */
export function discoveredLanCandidate(host: unknown): LanBootstrapCandidate | null {
  if (typeof host !== "object" || host === null || Array.isArray(host)) return null;
  const record = host as Record<string, unknown>;
  if (Object.keys(record).some(key => !["id", "displayName", "ipv4", "port"].includes(key))) return null;
  const { id, displayName, ipv4: address, port } = record;
  const ipv4 = exactPrivateAddress(address);
  if (typeof id !== "string" || !DISCOVERY_ID.test(id) ||
      !validDisplayName(displayName) || !validPort(port) || ipv4 === null) return null;
  return { kind: "discovered", deviceId: id, displayName, ipv4, port, trust: "unverified" };
}

/**
 * The strict IPv4 subset of protocol/pairing-wire.ts QR v1.
 * "host_key" is a public-key PIN, not a signed approval or reusable secret.
 * IPv6 ULA requires an explicit later native implementation.
 */
export function hostQrCandidate(raw: unknown): LanBootstrapCandidate | null {
  if (typeof raw !== "string" || raw.length > 256 || !raw.startsWith(QR_PREFIX)) return null;
  const components = raw.slice(QR_PREFIX.length).split("&");
  if (components.length !== 3) return null;
  const values = new Map<string, string>();
  for (const component of components) {
    const match = /^([a-z_]+)=([^&#?%]+)$/.exec(component);
    if (!match || match[1] === undefined || match[2] === undefined ||
        !VALID_FIELDS.has(match[1]) || values.has(match[1])) return null;
    values.set(match[1], match[2]);
  }
  const ipv4 = exactPrivateAddress(values.get("ip"));
  const portString = values.get("port");
  const hostPublicKeyHex = values.get("host_key");
  if (ipv4 === null || portString === undefined || !/^[1-9][0-9]{0,4}$/.test(portString) ||
      hostPublicKeyHex === undefined || !PUBLIC_KEY_HEX.test(hostPublicKeyHex) ||
      /^0+$/.test(hostPublicKeyHex)) return null;
  const port = Number(portString);
  if (!validPort(port)) return null;
  return { kind: "qr-host-key-pin", ipv4, port, hostPublicKeyHex, trust: "key-pin-only" };
}

/** A JS candidate alone never authorizes input. Native session binding is absent. */
export function canSendRemoteInputFromBootstrap(candidate: LanBootstrapCandidate): false {
  switch (candidate.kind) {
    case "manual-ip":
    case "discovered":
    case "qr-host-key-pin":
      return false;
  }
}

/** Shape expected from future authenticated native mDNS adapter (not installed). */
export type DiscoveredHostHint = DiscoveredHost;
