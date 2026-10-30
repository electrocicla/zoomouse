import { describe, expect, it } from "vitest";
import { PROTOCOL_VERSION } from "./index";
import {
  authorizeInputFrame, initialInputSessionState, localConsentSchema,
  pairingIntentSchema, parseInputFrame, releaseHeldInputs, SESSION_POLICY,
  sessionNoticeSchema
} from "@zoomouse/protocol/session-policy";
import type { InputSessionState, TrustedSessionContext } from "@zoomouse/protocol/session-policy";

const NOW = 1_770_000_000_000;
const SESSION = "session-test-aaaaaaaaaaaaaaaa";
const DEVICE = "device-test-aaaaaaaaaaaaaaaa";
const BINDING = "binding-test-aaaaaaaaaaaaaaaa";
const USER = "windows-user-aaaaaaaaaaaaaaaa";
const CONSENT = {
  consentId: "consent-test-aaaaaaaaaaaaaaaa",
  requestId: "request-test-aaaaaaaaaaaaaaaa",
  deviceId: DEVICE, sessionId: SESSION,
  transportBindingId: BINDING, windowsUserSessionId: USER,
  permission: "remote-input" as const, generation: 1,
  issuedAtMs: NOW - 1000, expiresAtMs: NOW + 300_000
};
const AUTH: TrustedSessionContext = {
  sessionId: SESSION, deviceId: DEVICE, authenticatedDeviceId: DEVICE,
  transportBindingId: BINDING, windowsUserSessionId: USER,
  transportAuthenticated: true, connected: true, interactiveDesktop: true,
  revoked: false, currentConsentGeneration: 1, consent: CONSENT
};

function wire(sequence: number, fields: Record<string, unknown> = {}, timestampMs = NOW): string {
  const type = fields.type ?? "pointer.move";
  const defaults = type === "pointer.move" ? { dx: 1, dy: -1 } : {};
  return JSON.stringify({
    version: PROTOCOL_VERSION, sequence, timestampMs, sessionId: SESSION,
    type, ...defaults, ...fields
  });
}

function send(
  sequence: number, state: InputSessionState, fields: Record<string, unknown> = {},
  ctx: TrustedSessionContext = AUTH, time = NOW
) {
  return authorizeInputFrame(wire(sequence, fields), ctx, state, time);
}

function accepted(sequence: number, state: InputSessionState, fields: Record<string, unknown> = {}, time = NOW) {
  const result = send(sequence, state, fields, AUTH, time);
  expect(result.accepted).toBe(true);
  return result.next;
}

describe("M1-A wire and consent contracts", () => {
  it("validates explicit v1 pairing intent and informational notice without extraneous fields", () => {
    const intent = {
      version: PROTOCOL_VERSION, type: "pairing.intent",
      requestId: CONSENT.requestId, deviceId: DEVICE,
      displayName: "My phone", clientNonce: "abcdef0123456789abcdef0123456789"
    };
    expect(pairingIntentSchema.safeParse(intent).success).toBe(true);
    expect(pairingIntentSchema.safeParse({ ...intent, version: 2 }).success).toBe(false);
    expect(pairingIntentSchema.safeParse({ ...intent, command: "powershell" }).success).toBe(false);
    expect(pairingIntentSchema.safeParse({ ...intent, clientNonce: "" }).success).toBe(false);
    const notice = {
      version: PROTOCOL_VERSION, type: "session.notice", requestId: CONSENT.requestId,
      deviceId: DEVICE, sessionId: SESSION, expiresAtMs: NOW + 300_000, consentGeneration: 1
    };
    expect(sessionNoticeSchema.safeParse(notice).success).toBe(true);
    expect(sessionNoticeSchema.safeParse({ ...notice, permission: "remote-input" }).success).toBe(false);
    expect(localConsentSchema.safeParse(CONSENT).success).toBe(true);
    expect(localConsentSchema.safeParse({ ...CONSENT, privilege: "admin" }).success).toBe(false);
  });

  it("rejects malformed, oversized, extra-field and unsupported-version frames", () => {
    expect(parseInputFrame("{").ok).toBe(false);
    expect(parseInputFrame(null)).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame({ type: "pointer.move" })).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame("x".repeat(SESSION_POLICY.maxWireBytes + 1))).toEqual({ ok: false, reason: "frame_too_large" });
    expect(parseInputFrame(wire(1, { version: 2 }))).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame(wire(1, { command: "exec" }))).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame(wire(1, { dx: Infinity }))).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame(wire(1, { dx: Number.MAX_SAFE_INTEGER }))).toEqual({ ok: false, reason: "invalid_packet" });
    expect(parseInputFrame("🙂".repeat(1025))).toEqual({ ok: false, reason: "frame_too_large" });
  });
});

describe("M1-A authenticated host authorization (pure policy only)", () => {
  it("fails closed for unauthenticated, unmatched, revoked or unapproved clients", () => {
    const empty = initialInputSessionState();
    const denied: TrustedSessionContext[] = [
      { ...AUTH, authenticatedDeviceId: null },
      { ...AUTH, transportAuthenticated: false },
      { ...AUTH, transportBindingId: "binding-other-aaaaaaaaaaaaaaaa" },
      { ...AUTH, connected: false },
      { ...AUTH, interactiveDesktop: false },
      { ...AUTH, revoked: true },
      { ...AUTH, currentConsentGeneration: 2 },
      { ...AUTH, currentConsentGeneration: -1 },
      { ...AUTH, currentConsentGeneration: Number.MAX_SAFE_INTEGER + 1 },
      { ...AUTH, consent: { ...CONSENT, generation: 2 } },
      { ...AUTH, consent: null },
      { ...AUTH, consent: { ...CONSENT, permission: "view-only" } },
      { ...AUTH, consent: { ...CONSENT, windowsUserSessionId: "other-user-aaaaaaaaaaaaaaaa" } }
    ];
    for (const ctx of denied) {
      const result = send(1, empty, {}, ctx);
      expect(result).toEqual({ accepted: false, reason: "unauthorized", next: empty });
    }
  });

  it("fences stale consent across host revocation and explicit re-approval", () => {
    const initial = initialInputSessionState();
    const afterRevocation: TrustedSessionContext = { ...AUTH, currentConsentGeneration: 2 };
    expect(send(1, initial, {}, afterRevocation)).toMatchObject({ reason: "unauthorized", next: initial });

    const renewed: TrustedSessionContext = {
      ...afterRevocation,
      consent: { ...CONSENT, consentId: "consent-test-bbbbbbbbbbbbbbbb", generation: 2 }
    };
    expect(send(1, initial, {}, renewed).accepted).toBe(true);
    expect(send(1, initial, {}, { ...AUTH, consent: renewed.consent })).toMatchObject({ reason: "unauthorized" });
  });

  it("rejects mismatched session, expired, excessively long and future consent", () => {
    const empty = initialInputSessionState();
    expect(send(1, empty, { sessionId: "session-other-aaaaaaaaaaaaaaaa" }).accepted).toBe(false);
    expect(send(1, empty, {}, { ...AUTH, consent: { ...CONSENT, expiresAtMs: NOW } })).toMatchObject({ reason: "consent_expired" });
    expect(send(1, empty, {}, { ...AUTH, consent: { ...CONSENT, expiresAtMs: NOW + 999_999 } })).toMatchObject({ reason: "consent_expired" });
    expect(send(1, empty, {}, { ...AUTH, consent: { ...CONSENT, issuedAtMs: NOW + 1 } })).toMatchObject({ reason: "consent_expired" });
  });

  it("rejects stale/future packets, invalid host time and backwards time", () => {
    const empty = initialInputSessionState();
    expect(authorizeInputFrame(wire(1), AUTH, empty, Number.NaN)).toMatchObject({ reason: "invalid_clock" });
    expect(authorizeInputFrame(wire(1, {}, NOW - 5001), AUTH, empty, NOW)).toMatchObject({ reason: "packet_stale" });
    expect(authorizeInputFrame(wire(1, {}, NOW + 2001), AUTH, empty, NOW)).toMatchObject({ reason: "packet_from_future" });
    const state = accepted(1, empty);
    expect(authorizeInputFrame(wire(2), AUTH, state, NOW - 1)).toMatchObject({ reason: "clock_regressed" });
  });

  it("enforces first=1, strict monotonicity, replay bit window and bounded gaps", () => {
    const initial = initialInputSessionState();
    expect(send(0, initial)).toMatchObject({ reason: "sequence_first" });
    expect(send(10, initial)).toMatchObject({ reason: "sequence_first" });
    const afterFirst = accepted(1, initial);
    expect(send(1, afterFirst)).toMatchObject({ reason: "sequence_replayed", next: afterFirst });
    expect(send(0, afterFirst)).toMatchObject({ reason: "sequence_out_of_order" });
    expect(send(100, afterFirst)).toMatchObject({ reason: "sequence_gap" });
    const advanced = accepted(3, afterFirst);
    expect(send(2, advanced)).toMatchObject({ reason: "sequence_out_of_order" });
    expect(send(3, advanced)).toMatchObject({ reason: "sequence_replayed" });
    let state = initial;
    for (let i = 1; i <= 80; i += 1) state = accepted(i, state, { type: "pointer.scroll", dx: 1, dy: 1 });
    expect(send(1, state)).toMatchObject({ reason: "sequence_too_old" });
  });

  it("enforces per-window pointer, text and aggregate rate limits with recovery", () => {
    let moves = initialInputSessionState();
    for (let i = 1; i <= SESSION_POLICY.maxMovesPerWindow; i += 1) moves = accepted(i, moves);
    expect(send(61, moves)).toMatchObject({ reason: "rate_exceeded", next: moves });
    const reset = authorizeInputFrame(wire(61, {}, NOW + 1000), AUTH, moves, NOW + 1000);
    expect(reset.accepted).toBe(true);
    let textState = initialInputSessionState();
    for (let i = 1; i <= SESSION_POLICY.maxTextPerWindow; i += 1) {
      textState = accepted(i, textState, { type: "keyboard.text", text: "hello" });
    }
    expect(send(6, textState, { type: "keyboard.text", text: "hello" })).toMatchObject({ reason: "rate_exceeded" });
    let scrollState = initialInputSessionState();
    for (let i = 1; i <= SESSION_POLICY.maxEventsPerWindow; i += 1) {
      scrollState = accepted(i, scrollState, { type: "pointer.scroll", dx: 0, dy: 1 });
    }
    expect(send(91, scrollState, { type: "pointer.scroll", dx: 0, dy: 1 })).toMatchObject({ reason: "rate_exceeded" });
  });

  it("tracks held keys/buttons, rejects invalid input state and returns releases on explicit releaseAll", () => {
    const initial = initialInputSessionState();
    const key = { type: "keyboard.key", code: "KeyQ", state: "down", modifiers: [] };
    const afterKey = accepted(1, initial, key);
    expect(send(2, afterKey, key)).toMatchObject({ reason: "input_state_conflict" });
    expect(send(2, afterKey, { ...key, code: "Power" })).toMatchObject({ reason: "unsupported_key" });
    const withButton = accepted(2, afterKey, { type: "pointer.button", button: "left", state: "down" });
    const released = send(3, withButton, { type: "input.releaseAll" });
    expect(released).toMatchObject({ accepted: true, release: { keys: ["KeyQ"], buttons: ["left"] } });
    expect(released.next.heldKeys).toEqual([]);
    expect(released.next.heldButtons).toEqual([]);
    expect(send(4, released.next, { type: "keyboard.key", code: "KeyQ", state: "up", modifiers: [] })).toMatchObject({ reason: "input_state_conflict" });
  });

  it("releases held input on revoked session or disconnect without treating release as network authorization", () => {
    const keyState = accepted(1, initialInputSessionState(), { type: "keyboard.key", code: "KeyA", state: "down", modifiers: [] });
    const held = accepted(2, keyState, { type: "pointer.button", button: "right", state: "down" });
    expect(send(3, held, {}, { ...AUTH, revoked: true })).toMatchObject({ reason: "unauthorized" });
    const cleanup = releaseHeldInputs(held, "disconnect");
    expect(cleanup.release).toEqual({ keys: ["KeyA"], buttons: ["right"] });
    expect(cleanup.next).toMatchObject({ terminated: true, highestSequence: 0, heldKeys: [], heldButtons: [] });
    expect(send(1, cleanup.next)).toMatchObject({ reason: "session_terminated" });
    expect(releaseHeldInputs(cleanup.next, "revoked").release).toEqual({ keys: [], buttons: [] });
  });
});
