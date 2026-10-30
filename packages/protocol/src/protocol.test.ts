import { describe, expect, it } from "vitest";
import {
  inputPacketSchema,
  peerAdvertisementSchema,
  pointerMoveSchema,
  zoomSettingsSchema,
  PROTOCOL_VERSION
} from "./index";

const base = {
  version: PROTOCOL_VERSION,
  sequence: 1,
  timestampMs: 1700000000000,
  sessionId: "session-aaaaaaaaaaaaaaaa"
} as const;

describe("Zoomouse protocol safety", () => {
  it("accepts bounded pointer movement", () => {
    expect(pointerMoveSchema.safeParse({
      ...base,
      type: "pointer.move",
      dx: 12,
      dy: -2
    }).success).toBe(true);
  });

  it("rejects excessive pointer delta, NaN, extra properties and unknown versions", () => {
    expect(pointerMoveSchema.safeParse({ ...base, type: "pointer.move", dx: 90000, dy: 0 }).success).toBe(false);
    expect(pointerMoveSchema.safeParse({ ...base, type: "pointer.move", dx: Number.NaN, dy: 0 }).success).toBe(false);
    expect(pointerMoveSchema.safeParse({ ...base, type: "pointer.move", dx: 1, dy: 0, command: "exec" }).success).toBe(false);
    expect(pointerMoveSchema.safeParse({ ...base, version: 2, type: "pointer.move", dx: 1, dy: 0 }).success).toBe(false);
  });

  it("requires strict keyboard keys and capped text", () => {
    expect(inputPacketSchema.safeParse({ ...base, type: "keyboard.key", code: "KeyA", state: "up", modifiers: [] }).success).toBe(true);
    expect(inputPacketSchema.safeParse({ ...base, type: "keyboard.text", text: "x".repeat(3000) }).success).toBe(false);
  });

  it("enforces zoom invariants", () => {
    const good = { enabled: true, followCursor: true, magnification: 2.5, smoothing: 0.6, maxFps: 30 };
    expect(zoomSettingsSchema.parse(good).magnification).toBe(2.5);
    expect(zoomSettingsSchema.safeParse({ ...good, magnification: 0 }).success).toBe(false);
    expect(zoomSettingsSchema.safeParse({ ...good, magnification: 8 }).success).toBe(false);
  });

  it("requires a valid port and protocol identity", () => {
    const ad = { protocolVersion: PROTOCOL_VERSION, deviceId: "device-abcdefghijkl", hostName: "Windows-11", address: "192.168.1.2", port: 45000 };
    expect(peerAdvertisementSchema.safeParse(ad).success).toBe(true);
    expect(peerAdvertisementSchema.safeParse({ ...ad, port: 70000 }).success).toBe(false);
  });
});
