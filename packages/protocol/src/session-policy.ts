import { z } from "zod";
import { inputPacketSchema, PROTOCOL_VERSION } from "./index";

const opaqueIdSchema = z.string().min(16).max(128);
const hostTimeSchema = z.number().int().nonnegative().safe();

/** Untrusted LAN offer. Neither a deviceId nor a public-key claim authenticates its sender. */
export const pairingIntentSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("pairing.intent"),
  requestId: opaqueIdSchema,
  deviceId: opaqueIdSchema,
  displayName: z.string().trim().min(1).max(128),
  clientNonce: z.string().regex(/^[A-Za-z0-9_-]{22,128}$/)
});

/** An informational server reply. A client must not treat this as proof of authorization. */
export const sessionNoticeSchema = z.strictObject({
  version: z.literal(PROTOCOL_VERSION),
  type: z.literal("session.notice"),
  requestId: opaqueIdSchema,
  deviceId: opaqueIdSchema,
  sessionId: opaqueIdSchema,
  expiresAtMs: hostTimeSchema,
  consentGeneration: z.number().int().nonnegative().safe()
});

/** Host-local record ONLY: must never be populated from a DataChannel or discovery payload. */
export const localConsentSchema = z.strictObject({
  consentId: opaqueIdSchema,
  requestId: opaqueIdSchema,
  deviceId: opaqueIdSchema,
  sessionId: opaqueIdSchema,
  transportBindingId: opaqueIdSchema,
  windowsUserSessionId: opaqueIdSchema,
  permission: z.enum(["remote-input", "view-only", "denied"]),
  generation: z.number().int().nonnegative().safe(),
  issuedAtMs: hostTimeSchema,
  expiresAtMs: hostTimeSchema
});

export const SESSION_POLICY = Object.freeze({
  maxWireBytes: 4096,
  maxPacketAgeMs: 5000,
  maxFutureSkewMs: 2000,
  maxConsentLeaseMs: 600000,
  replayWindowSize: 64,
  maxSequenceAdvance: 64,
  rateWindowMs: 1000,
  maxEventsPerWindow: 90,
  maxMovesPerWindow: 60,
  maxTextPerWindow: 5,
  maxHeldKeys: 32
});

export type PairingIntent = z.infer<typeof pairingIntentSchema>;
export type SessionNotice = z.infer<typeof sessionNoticeSchema>;
export type LocalConsent = z.infer<typeof localConsentSchema>;

/**
 * A host-authored snapshot, not a wire schema. Only M1-D authenticated transport,
 * the native consent authority and the Windows user-session engine may construct it.
 * TypeScript types are NOT a cryptographic or process trust boundary.
 */
export interface TrustedSessionContext {
  readonly sessionId: string;
  readonly deviceId: string;
  readonly authenticatedDeviceId: string | null;
  readonly transportBindingId: string | null;
  readonly windowsUserSessionId: string;
  readonly transportAuthenticated: boolean;
  readonly connected: boolean;
  readonly interactiveDesktop: boolean;
  readonly revoked: boolean;
  /** Trusted current host consent epoch; increment on revoke, expiry or re-approval. */
  readonly currentConsentGeneration: number;
  readonly consent: LocalConsent | null;
}

type Button = "left" | "right" | "middle";
export type ReleaseReason = "disconnect" | "reconnect" | "revoked" | "expired" | "desktop-locked" | "dispatch-failed";

export interface InputSessionState {
  readonly terminated: boolean;
  readonly highestSequence: number;
  readonly recentSequenceBits: bigint;
  readonly windowStartedAtMs: number | null;
  readonly lastHostNowMs: number | null;
  readonly eventCount: number;
  readonly moveCount: number;
  readonly textCount: number;
  readonly heldKeys: readonly string[];
  readonly heldButtons: readonly Button[];
}

export interface ReleaseDirective {
  readonly keys: readonly string[];
  readonly buttons: readonly Button[];
}

export type RejectionReason =
  | "frame_too_large" | "invalid_json" | "invalid_packet"
  | "invalid_clock" | "clock_regressed" | "unauthorized" | "consent_expired"
  | "wrong_session" | "packet_stale" | "packet_from_future"
  | "sequence_first" | "sequence_replayed" | "sequence_out_of_order"
  | "sequence_too_old" | "sequence_gap" | "session_terminated" | "rate_exceeded"
  | "unsupported_key" | "input_state_conflict" | "held_key_limit";

export type InputDecision =
  | { readonly accepted: true; readonly packet: z.infer<typeof inputPacketSchema>; readonly next: InputSessionState; readonly release: ReleaseDirective }
  | { readonly accepted: false; readonly reason: RejectionReason; readonly next: InputSessionState };

/** Sequence numbers start at 1 for every newly authenticated session; zero is invalid. */
export function initialInputSessionState(): InputSessionState {
  return {
    terminated: false, highestSequence: 0, recentSequenceBits: 0n,
    windowStartedAtMs: null, lastHostNowMs: null,
    eventCount: 0, moveCount: 0, textCount: 0,
    heldKeys: [], heldButtons: []
  };
}

/** Release is a host-owned best-effort native operation, not a wire command. */
export function releaseHeldInputs(state: InputSessionState, reason: ReleaseReason): {
  readonly reason: ReleaseReason;
  readonly release: ReleaseDirective;
  readonly next: InputSessionState;
} {
  return {
    reason,
    release: { keys: [...state.heldKeys], buttons: [...state.heldButtons] },
    next: { ...initialInputSessionState(), terminated: true }
  };
}

/** Counts encoded UTF-8 bytes of a raw JSON text frame without relying on browser/Node APIs. */
function utf8Bytes(raw: string): number {
  let bytes = 0;
  for (let i = 0; i < raw.length; i += 1) {
    const code = raw.charCodeAt(i);
    if (code < 0x80) bytes += 1;
    else if (code < 0x800) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff && i + 1 < raw.length) {
      const low = raw.charCodeAt(i + 1);
      if (low >= 0xdc00 && low <= 0xdfff) { bytes += 4; i += 1; }
      else bytes += 3;
    } else bytes += 3;
    if (bytes > SESSION_POLICY.maxWireBytes) return bytes;
  }
  return bytes;
}

type ParsedFrame =
  | { readonly ok: true; readonly packet: z.infer<typeof inputPacketSchema> }
  | { readonly ok: false; readonly reason: RejectionReason };

/** Only a raw text payload from the already-authenticated channel is admitted. */
export function parseInputFrame(raw: unknown): ParsedFrame {
  if (typeof raw !== "string") return { ok: false, reason: "invalid_packet" };
  if (utf8Bytes(raw) > SESSION_POLICY.maxWireBytes) {
    return { ok: false, reason: "frame_too_large" };
  }
  let decoded: unknown;
  try { decoded = JSON.parse(raw) as unknown; }
  catch { return { ok: false, reason: "invalid_json" }; }
  const result = inputPacketSchema.safeParse(decoded);
  if (!result.success) return { ok: false, reason: "invalid_packet" };
  return { ok: true, packet: result.data };
}

const codePattern = /^(?:Key[A-Z]|Digit[0-9]|Numpad[0-9]|Arrow(?:Up|Down|Left|Right)|Enter|Escape|Backspace|Delete|Tab|Space|Home|End|PageUp|PageDown|Insert|Shift(?:Left|Right)|Control(?:Left|Right)|Alt(?:Left|Right)|Meta(?:Left|Right)|F(?:[1-9]|1[0-2]))$/;
const bitmapMask = (1n << BigInt(SESSION_POLICY.replayWindowSize)) - 1n;
const emptyRelease: ReleaseDirective = { keys: [], buttons: [] };

function rejected(reason: RejectionReason, state: InputSessionState): InputDecision {
  return { accepted: false, reason, next: state };
}

function hasHostAuthority(ctx: TrustedSessionContext, nowMs: number): "ok" | "unauthorized" | "consent_expired" {
  if (!ctx.connected || !ctx.interactiveDesktop || ctx.revoked ||
      !ctx.transportAuthenticated || !ctx.authenticatedDeviceId || !ctx.transportBindingId) {
    return "unauthorized";
  }
  const consent = ctx.consent;
  if (!consent || !localConsentSchema.safeParse(consent).success ||
      !hostTimeSchema.safeParse(ctx.currentConsentGeneration).success ||
      consent.generation !== ctx.currentConsentGeneration ||
      consent.permission !== "remote-input" ||
      ctx.deviceId !== ctx.authenticatedDeviceId ||
      consent.deviceId !== ctx.deviceId || consent.sessionId !== ctx.sessionId ||
      consent.transportBindingId !== ctx.transportBindingId ||
      consent.windowsUserSessionId !== ctx.windowsUserSessionId) {
    return "unauthorized";
  }
  if (consent.expiresAtMs <= nowMs || consent.issuedAtMs > nowMs ||
      consent.expiresAtMs <= consent.issuedAtMs ||
      consent.expiresAtMs - consent.issuedAtMs > SESSION_POLICY.maxConsentLeaseMs) {
    return "consent_expired";
  }
  return "ok";
}

/**
 * Pure gate: caller obtains ctx from trusted native sources, never from JSON.
 * Return next state is proposed, not evidence that OS dispatch succeeded.
 * On native dispatch failure, revoke the session and apply releaseHeldInputs.
 */
export function authorizeInputFrame(
  raw: unknown, ctx: TrustedSessionContext, state: InputSessionState, nowMs: number
): InputDecision {
  if (state.terminated) return rejected("session_terminated", state);
  if (!hostTimeSchema.safeParse(nowMs).success) return rejected("invalid_clock", state);
  if (state.lastHostNowMs !== null && nowMs < state.lastHostNowMs) return rejected("clock_regressed", state);
  const frame = parseInputFrame(raw);
  if (!frame.ok) return rejected(frame.reason, state);
  const packet = frame.packet;
  if (packet.sessionId !== ctx.sessionId) return rejected("wrong_session", state);
  const authority = hasHostAuthority(ctx, nowMs);
  if (authority !== "ok") return rejected(authority, state);
  if (packet.timestampMs > nowMs + SESSION_POLICY.maxFutureSkewMs) return rejected("packet_from_future", state);
  if (nowMs - packet.timestampMs > SESSION_POLICY.maxPacketAgeMs) return rejected("packet_stale", state);
  if (packet.sequence === 0 && state.highestSequence === 0) return rejected("sequence_first", state);
  if (state.highestSequence === 0 && packet.sequence !== 1) return rejected("sequence_first", state);
  if (packet.sequence <= state.highestSequence) {
    const distance = state.highestSequence - packet.sequence;
    if (distance >= SESSION_POLICY.replayWindowSize) return rejected("sequence_too_old", state);
    const seen = (state.recentSequenceBits & (1n << BigInt(distance))) !== 0n;
    return rejected(seen ? "sequence_replayed" : "sequence_out_of_order", state);
  }
  const advance = packet.sequence - state.highestSequence;
  if (advance > SESSION_POLICY.maxSequenceAdvance) return rejected("sequence_gap", state);

  const windowStart = state.windowStartedAtMs;
  const freshWindow = windowStart === null || nowMs - windowStart >= SESSION_POLICY.rateWindowMs;
  const eventCount = (freshWindow ? 0 : state.eventCount) + 1;
  const moveCount = (freshWindow ? 0 : state.moveCount) + (packet.type === "pointer.move" ? 1 : 0);
  const textCount = (freshWindow ? 0 : state.textCount) + (packet.type === "keyboard.text" ? 1 : 0);
  if (eventCount > SESSION_POLICY.maxEventsPerWindow ||
      moveCount > SESSION_POLICY.maxMovesPerWindow || textCount > SESSION_POLICY.maxTextPerWindow) {
    return rejected("rate_exceeded", state);
  }

  let heldKeys: readonly string[] = state.heldKeys;
  let heldButtons: readonly Button[] = state.heldButtons;
  let release: ReleaseDirective = emptyRelease;
  if (packet.type === "keyboard.key") {
    if (!codePattern.test(packet.code)) return rejected("unsupported_key", state);
    const alreadyHeld = heldKeys.includes(packet.code);
    if (packet.state === "down") {
      if (alreadyHeld) return rejected("input_state_conflict", state);
      if (heldKeys.length >= SESSION_POLICY.maxHeldKeys) return rejected("held_key_limit", state);
      heldKeys = [...heldKeys, packet.code];
    } else {
      if (!alreadyHeld) return rejected("input_state_conflict", state);
      heldKeys = heldKeys.filter((key) => key !== packet.code);
    }
  } else if (packet.type === "pointer.button") {
    const alreadyHeld = heldButtons.includes(packet.button);
    if ((packet.state === "down") === alreadyHeld) return rejected("input_state_conflict", state);
    heldButtons = packet.state === "down"
      ? [...heldButtons, packet.button]
      : heldButtons.filter((button) => button !== packet.button);
  } else if (packet.type === "input.releaseAll") {
    release = { keys: [...heldKeys], buttons: [...heldButtons] };
    heldKeys = [];
    heldButtons = [];
  } else if (packet.type === "keyboard.text" && packet.text.length === 0) {
    return rejected("invalid_packet", state);
  }

  const next: InputSessionState = {
    terminated: false, highestSequence: packet.sequence,
    recentSequenceBits: ((state.recentSequenceBits << BigInt(advance)) | 1n) & bitmapMask,
    windowStartedAtMs: freshWindow ? nowMs : windowStart,
    lastHostNowMs: nowMs,
    eventCount, moveCount, textCount,
    heldKeys, heldButtons
  };
  return { accepted: true, packet, next, release };
}
