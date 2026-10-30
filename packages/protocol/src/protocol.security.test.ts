import { describe, expect, it } from "vitest";
import {
  captureSourceSchema,
  inputPacketSchema,
  keyboardKeySchema,
  peerAdvertisementSchema,
  pointerButtonSchema,
  releaseAllSchema,
  scrollSchema,
  sessionStateSchema,
  zoomSettingsSchema,
  PROTOCOL_VERSION
} from "./index";

const base = {
  version: PROTOCOL_VERSION,
  sequence: 42,
  timestampMs: 1761790000000,
  sessionId: "session-0123456789abcdef"
} as const;

describe("protocol defensive validation", () => {
  it("rejects unknown packet types and arbitrary extra commands", () => {
    expect(inputPacketSchema.safeParse({ ...base, type: "exec", command: "powershell" }).success).toBe(false);
    expect(releaseAllSchema.safeParse({ ...base, type: "input.releaseAll", extra: "shutdown" }).success).toBe(false);
  });

  it("rejects negative, fractional, and unsafe packet sequences", () => {
    for (const sequence of [-1, 1.1, Number.MAX_SAFE_INTEGER + 2]) {
      expect(pointerButtonSchema.safeParse({ ...base, sequence, type: "pointer.button", button: "left", state: "up" }).success).toBe(false);
    }
  });

  it("rejects scroll infinities and invalid button state", () => {
    expect(scrollSchema.safeParse({ ...base, type: "pointer.scroll", dx: Infinity, dy: 1 }).success).toBe(false);
    expect(pointerButtonSchema.safeParse({ ...base, type: "pointer.button", button: "primary", state: "up" }).success).toBe(false);
    expect(pointerButtonSchema.safeParse({ ...base, type: "pointer.button", button: "right", state: "hold" }).success).toBe(false);
  });

  it("validates keyboard modifiers and payload size", () => {
    const valid = { ...base, type: "keyboard.key", code: "KeyQ", state: "down", modifiers: ["ctrl"] };
    expect(keyboardKeySchema.safeParse(valid).success).toBe(true);
    expect(keyboardKeySchema.safeParse({ ...valid, modifiers: ["sudo"] }).success).toBe(false);
    expect(keyboardKeySchema.safeParse({ ...valid, modifiers: ["ctrl", "alt", "shift", "meta", "ctrl"] }).success).toBe(false);
  });

  it("requires bounded capture-source ids, zoom rates and valid stream state", () => {
    expect(captureSourceSchema.safeParse({ kind: "display", stableId: "DISPLAY-01" }).success).toBe(true);
    expect(captureSourceSchema.safeParse({ kind: "desktop", stableId: "DISPLAY-01" }).success).toBe(false);
    expect(zoomSettingsSchema.safeParse({ enabled: true, followCursor: true, magnification: 1, smoothing: 0, maxFps: 25 }).success).toBe(false);
    expect(sessionStateSchema.safeParse("connected").success).toBe(true);
    expect(sessionStateSchema.safeParse("authorized_forever").success).toBe(false);
  });

  it("rejects invalid discovery advertisement shapes", () => {
    const ad = { protocolVersion: PROTOCOL_VERSION, deviceId: "device-0123456789abcdef", hostName: "Host", address: "192.168.0.2", port: 12345 };
    expect(peerAdvertisementSchema.safeParse(ad).success).toBe(true);
    expect(peerAdvertisementSchema.safeParse({ ...ad, protocolVersion: 99 }).success).toBe(false);
    expect(peerAdvertisementSchema.safeParse({ ...ad, unknown: "field" }).success).toBe(false);
  });
});
