import assert from "node:assert/strict";
import test from "node:test";
import { NATIVE_APPROVAL_GATE, parseHostStatus } from "./src/host-status.ts";

const denied = Object.freeze({
  phase: "Denied",
  inputEnabled: false,
  cleanupFailed: false,
  discoveryEnabled: false,
  captureEnabled: false,
  localUserSessionOnly: false,
  approvalGate: NATIVE_APPROVAL_GATE,
  lastReleaseOutcome: "notAttempted"
});

test("local shell is denied and native release outcome is observable", () => {
  assert.deepEqual(parseHostStatus(denied), denied);
  const revoked = { ...denied, phase: "Revoked", lastReleaseOutcome: "confirmed" };
  assert.deepEqual(parseHostStatus(revoked), revoked);
  const failed = { ...revoked, lastReleaseOutcome: "failed", cleanupFailed: true };
  assert.deepEqual(parseHostStatus(failed), failed);
});

test("malformed IPC payloads remain unverified", () => {
  for (const value of [
    null, undefined, [], "Approved", 1, {}, { ...denied, phase: "SomethingElse" },
    { ...denied, inputEnabled: "true" }, { ...denied, discoveryEnabled: null },
    { ...denied, localUserSessionOnly: undefined }, { ...denied, captureEnabled: 1 },
    { ...denied, approvalGate: undefined }, { ...denied, lastReleaseOutcome: "unknown" }
  ]) {
    assert.equal(parseHostStatus(value), null);
  }
});

test("renderer rejects fabricated consent/channel/desktop/input claims", () => {
  for (const override of [
    { phase: "Approved" },
    { phase: "Pending" },
    { inputEnabled: true },
    { discoveryEnabled: true },
    { captureEnabled: true },
    { localUserSessionOnly: true },
    { approvalGate: "ready" }
  ]) {
    assert.equal(parseHostStatus({ ...denied, ...override }), null);
  }
});

test("release state cannot mask failure or contradict native revocation", () => {
  for (const value of [
    { ...denied, cleanupFailed: false, lastReleaseOutcome: "failed" },
    { ...denied, phase: "Denied", lastReleaseOutcome: "confirmed" },
    { ...denied, phase: "Denied", lastReleaseOutcome: "failed", cleanupFailed: true },
    { ...denied, phase: "Revoked", lastReleaseOutcome: "failed", cleanupFailed: false }
  ]) {
    assert.equal(parseHostStatus(value), null);
  }
});
