import { z } from "zod";

/** Increment when the wire format changes incompatibly. */
export const PROTOCOL_VERSION = 1 as const;

const sequenceSchema = z.number().int().nonnegative().safe();
const packetBase = {
  version: z.literal(PROTOCOL_VERSION),
  sequence: sequenceSchema,
  timestampMs: z.number().int().nonnegative().safe(),
  sessionId: z.string().min(16).max(128)
};

export const pointerMoveSchema = z.strictObject({
  ...packetBase,
  type: z.literal("pointer.move"),
  dx: z.number().finite().min(-8192).max(8192),
  dy: z.number().finite().min(-8192).max(8192)
});

export const pointerButtonSchema = z.strictObject({
  ...packetBase,
  type: z.literal("pointer.button"),
  button: z.enum(["left", "right", "middle"]),
  state: z.enum(["down", "up"])
});

export const scrollSchema = z.strictObject({
  ...packetBase,
  type: z.literal("pointer.scroll"),
  dx: z.number().finite().min(-2000).max(2000),
  dy: z.number().finite().min(-2000).max(2000)
});

export const keyboardKeySchema = z.strictObject({
  ...packetBase,
  type: z.literal("keyboard.key"),
  code: z.string().min(1).max(64),
  state: z.enum(["down", "up"]),
  modifiers: z.array(z.enum(["ctrl", "alt", "shift", "meta"])).max(4)
});

export const keyboardTextSchema = z.strictObject({
  ...packetBase,
  type: z.literal("keyboard.text"),
  text: z.string().max(2048)
});

export const releaseAllSchema = z.strictObject({
  ...packetBase,
  type: z.literal("input.releaseAll")
});

export const inputPacketSchema = z.discriminatedUnion("type", [
  pointerMoveSchema,
  pointerButtonSchema,
  scrollSchema,
  keyboardKeySchema,
  keyboardTextSchema,
  releaseAllSchema
]);

export const captureSourceSchema = z.discriminatedUnion("kind", [
  z.strictObject({ kind: z.literal("display"), stableId: z.string().min(1).max(256) }),
  z.strictObject({ kind: z.literal("window"), stableId: z.string().min(1).max(256) })
]);

export const zoomSettingsSchema = z.strictObject({
  enabled: z.boolean(),
  followCursor: z.boolean(),
  magnification: z.number().finite().min(1).max(4),
  smoothing: z.number().finite().min(0).max(1),
  maxFps: z.union([z.literal(15), z.literal(30), z.literal(60)])
});

export const peerAdvertisementSchema = z.strictObject({
  protocolVersion: z.literal(PROTOCOL_VERSION),
  deviceId: z.string().min(16).max(128),
  hostName: z.string().min(1).max(128),
  address: z.string().min(1).max(255),
  port: z.number().int().min(1).max(65535)
});

export const sessionStateSchema = z.enum([
  "idle", "discovering", "pairing", "connected", "reconnecting", "disconnected"
]);

export type InputPacket = z.infer<typeof inputPacketSchema>;
export type CaptureSource = z.infer<typeof captureSourceSchema>;
export type ZoomSettings = z.infer<typeof zoomSettingsSchema>;
export type PeerAdvertisement = z.infer<typeof peerAdvertisementSchema>;
export type SessionState = z.infer<typeof sessionStateSchema>;
