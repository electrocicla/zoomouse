import { describe, expect, it } from "vitest";
import {
  canSendRemoteInputFromBootstrap,
  discoveredLanCandidate,
  hostQrCandidate,
  manualLanCandidate
} from "./lan-bootstrap";

const hostKey = "1".repeat(64);
const qr = (ip: string, port: string, key: string = hostKey): string =>
  `zoomouse://pair/v1?ip=${ip}&port=${port}&host_key=${key}`;

describe("manual IPv4 private-LAN bootstrap hints", () => {
  it("normalizes only user-entered whitespace and keeps input disabled", () => {
    const target = manualLanCandidate(" 192.168.1.5 ", 49701);
    expect(target).toEqual({ kind: "manual-ip", ipv4: "192.168.1.5", port: 49701, trust: "unverified" });
    if (target !== null) expect(canSendRemoteInputFromBootstrap(target)).toBe(false);
  });
  it.each(["8.8.8.8", "127.0.0.1", "169.254.1.2", "192.168.1.255", "192.168.01.2", "localhost", "::1", "", 7, null])(
    "rejects non-private or invalid address %s", input => {
      expect(manualLanCandidate(input, 49701)).toBeNull();
    }
  );
  it.each([0, 65536, -1, 3.2, Number.NaN, Number.POSITIVE_INFINITY, "49701", undefined])(
    "rejects invalid port %s", port => {
      expect(manualLanCandidate("10.1.2.3", port)).toBeNull();
    }
  );
});

describe("mDNS discovery is only an untrusted hint", () => {
  const valid = { id: "host-id_01", displayName: "PC Sala", ipv4: "172.20.0.10", port: 34567 };
  it("accepts a bounded private candidate without granting input", () => {
    const hint = discoveredLanCandidate(valid);
    expect(hint).toEqual({ kind: "discovered", deviceId: valid.id, displayName: valid.displayName,
      ipv4: valid.ipv4, port: valid.port, trust: "unverified" });
    if (hint !== null) expect(canSendRemoteInputFromBootstrap(hint)).toBe(false);
  });
  it.each([
    { ...valid, ipv4: "8.8.4.4" },
    { ...valid, ipv4: " 172.20.0.10 " },
    { ...valid, id: "evil/id" },
    { ...valid, id: "" },
    { ...valid, displayName: "" },
    { ...valid, displayName: "😀".repeat(40) },
    { ...valid, displayName: "\ud800" },
    { ...valid, port: 0 },
    { ...valid, port: 100000 },
    { ...valid, injected: true },
    null, [], "record"
  ])("rejects invalid discovery record %#", record => {
    expect(discoveredLanCandidate(record)).toBeNull();
  });
});

describe("QR v1 host key pin (not a signed approval)", () => {
  it("parses a canonical unambiguous RFC1918 IPv4 pin and never authorizes input", () => {
    const candidate = hostQrCandidate(qr("192.168.1.50", "4521"));
    expect(candidate).toEqual({
      kind: "qr-host-key-pin", ipv4: "192.168.1.50", port: 4521,
      hostPublicKeyHex: hostKey, trust: "key-pin-only"
    });
    if (candidate !== null) expect(canSendRemoteInputFromBootstrap(candidate)).toBe(false);
  });
  it("accepts shuffled exact parameters, never duplicate names", () => {
    const candidate = hostQrCandidate(`zoomouse://pair/v1?host_key=${hostKey}&port=9999&ip=10.2.3.4`);
    expect(candidate?.kind).toBe("qr-host-key-pin");
    expect(hostQrCandidate(qr("10.2.3.4", "9999") + "&host_key=" + hostKey)).toBeNull();
  });
  it.each([
    qr("8.8.8.8", "4521"), qr("127.0.0.1", "4521"),
    qr("192.168.1.0", "4521"), qr("192.168.1.255", "4521"),
    qr("192.168.1.50", "0"), qr("192.168.1.50", "65536"),
    qr("192.168.1.50", "04521"), qr("192.168.1.50", "4521", "0".repeat(64)),
    qr("192.168.1.50", "4521", "A".repeat(64)),
    qr("192.168.1.50", "4521", "a".repeat(63)),
    qr(" 192.168.1.50", "4521"), qr("192.168.1.50%20", "4521"),
    qr("fc00::1", "4521"),
    qr("192.168.1.50", "4521") + "&extra=1",
    "http://example.com/?host_key=" + hostKey,
    "zoomouse://pair/v2?ip=10.0.0.2&port=1&host_key=" + hostKey,
    "zoomouse://pair/v1?ip=10.0.0.2&port=1&port=2",
    "zoomouse://pair/v1?" + "x".repeat(257),
    null, 123
  ])("rejects malformed, public, overlong or ambiguous QR %#", input => {
    expect(hostQrCandidate(input)).toBeNull();
  });
});
