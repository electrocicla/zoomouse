import assert from "node:assert/strict";
import test from "node:test";
import { parseHostStatus } from "./src/host-status.ts";

const denied = Object.freeze({
  phase: "Denied",
  inputEnabled: false,
  cleanupFailed: false,
  discoveryEnabled: false,
  captureEnabled: false,
  localUserSessionOnly: true
});

test("native denied and revoked states are explicit", () => {
  assert.deepEqual(parseHostStatus(denied), denied);
  assert.equal(parseHostStatus({ ...denied, phase: "Revoked", cleanupFailed: true })?.cleanupFailed, true);
});

test("malformed IPC payloads remain unverified rather than approved", () => {
  for (const value of [
    null, undefined, [], "Approved", 1, {}, { ...denied, phase: "SomethingElse" },
    { ...denied, inputEnabled: "true" }, { ...denied, discoveryEnabled: null },
    { ...denied, localUserSessionOnly: undefined }, { ...denied, captureEnabled: 1 }
  ]) {
    assert.equal(parseHostStatus(value), null);
  }
});

test("invalid active-input claims cannot appear as an approved local host", () => {
  for (const override of [
    { phase: "Denied" },
    { phase: "Pending" },
    { phase: "Revoked" },
    { cleanupFailed: true },
    { localUserSessionOnly: false }
  ]) {
    assert.equal(parseHostStatus({ ...denied, phase: "Approved", inputEnabled: true, ...override }), null);
  }
  const approved = { ...denied, phase: "Approved", inputEnabled: true };
  assert.deepEqual(parseHostStatus(approved), approved);
});

test("unknown desktop context and failed cleanup cannot imply active service or approval", () => {
  const unverified = { ...denied, localUserSessionOnly: false };
  assert.deepEqual(parseHostStatus(unverified), unverified);
  for (const status of [
    { ...unverified, phase: "Approved" },
    { ...denied, phase: "Approved", cleanupFailed: true },
    { ...unverified, discoveryEnabled: true },
    { ...unverified, captureEnabled: true },
    { ...denied, cleanupFailed: true, discoveryEnabled: true },
    { ...denied, cleanupFailed: true, captureEnabled: true }
  ]) {
    assert.equal(parseHostStatus(status), null);
  }
});
