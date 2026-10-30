import { describe, expect, it } from "vitest";
import { connectionLabel, initialConnectionState, isInputAuthorized } from "./session";
import type { ConnectionState } from "./session";
import { createUnavailableLanClient, TransportNotImplementedError } from "./transport";

describe("mobile authorization boundary", () => {
  it("starts offline and cannot unlock during discovery or pending host approval", () => {
    expect(isInputAuthorized(initialConnectionState)).toBe(false);
    const discovering: ConnectionState = { phase: "discovering" };
    const pending: ConnectionState = {
      phase: "awaiting-host-approval",
      request: { kind: "pending-host-approval", requestId: "r1", hostFingerprint: "untrusted" },
    };
    expect(isInputAuthorized(discovering)).toBe(false);
    expect(isInputAuthorized(pending)).toBe(false);
    expect(connectionLabel(pending)).toContain("Windows");
  });

  it("rejects forged JS authorized phase until a native authenticated adapter exists", () => {
    const forged: ConnectionState = {
      phase: "authorized",
      session: {
        hostFingerprint: "spoofed",
        grantId: "forged-grant",
        sessionId: "forged-session",
        send: async () => undefined,
        disconnect: async () => undefined,
      },
    };
    expect(isInputAuthorized(forged)).toBe(false);
    expect(connectionLabel(forged)).toContain("sin transporte validado");
  });

  it("unavailable adapter cannot discover, approve, or return a session", async () => {
    const client = createUnavailableLanClient();
    const signal = new AbortController().signal;
    await expect(client.discover(signal)).rejects.toBeInstanceOf(TransportNotImplementedError);
    await expect(client.requestHostApproval({ kind: "manual-ip", ipv4: "192.168.1.5", port: 5000 }, signal)).rejects.toBeInstanceOf(TransportNotImplementedError);
    await expect(client.awaitApprovedSession("r1", signal)).rejects.toBeInstanceOf(TransportNotImplementedError);
  });
});
