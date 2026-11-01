import { describe, expect, it } from "vitest";
import {
  canSendInputFromLanAttempt,
  createUnavailableNativeLanPort,
  LanAttemptCoordinator,
  NativeLanUnavailableError
} from "./lan-attempt";
import type { LanAttemptState, NativeSecureLanPort } from "./lan-attempt";

const intent = { kind: "manual-ip", ipv4: "192.168.1.41", port: 4433 } as const;
const pin = "1".repeat(64);
const qrIntent = {
  kind: "qr", payload: `zoomouse://pair/v1?ip=10.0.0.4&port=4433&${["host", "key"].join("_")}=${pin}`
} as const;

function nativePort(
  open: NativeSecureLanPort["open"] = async () => undefined,
  close: NativeSecureLanPort["close"] = async () => undefined,
): NativeSecureLanPort { return { open, close }; }

describe("Android native LAN attempt boundary", () => {
  it("does not open a socket or authorize input in the default build", async () => {
    const coordinator = new LanAttemptCoordinator();
    expect(coordinator.state).toEqual({ phase: "idle" });
    const result = await coordinator.open(intent);
    expect(result).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
    expect(canSendInputFromLanAttempt(result)).toBe(false);
    await expect(createUnavailableNativeLanPort().open(1, {
      kind: "manual-ip", ipv4: "10.0.0.5", port: 4433, trust: "unverified"
    }, new AbortController().signal)).rejects.toBeInstanceOf(NativeLanUnavailableError);
  });

  it.each([
    { kind: "manual-ip" as const, ipv4: "8.8.8.8", port: 4433 },
    { kind: "manual-ip" as const, ipv4: "127.0.0.1", port: 4433 },
    { kind: "manual-ip" as const, ipv4: "192.168.1.255", port: 4433 },
    { kind: "manual-ip" as const, ipv4: "192.168.1.2", port: 0 },
    { kind: "manual-ip" as const, ipv4: "localhost", port: 4433 },
    { kind: "qr" as const, payload: "https://public.example/pair" },
    { kind: "qr" as const, payload: "zoomouse://pair/v1?ip=8.8.8.8&port=4433&host_key=" + pin },
    { kind: "discovered" as const, host: { id: "x", displayName: "Host", ipv4: "8.8.8.8", port: 4433 } },
  ])("refuses invalid and public network target %# before native open", async invalid => {
    let opens = 0;
    const coordinator = new LanAttemptCoordinator(nativePort(async () => { opens++; }));
    expect(await coordinator.open(invalid)).toEqual({ phase: "blocked", reason: "invalid-candidate" });
    expect(opens).toBe(0);
  });

  it("admits parsed QR as a key pin only and never grants send on native completion", async () => {
    let candidateKind = "";
    const coordinator = new LanAttemptCoordinator(nativePort(async (_, candidate) => {
      candidateKind = candidate.kind;
    }));
    const result = await coordinator.open(qrIntent);
    expect(candidateKind).toBe("qr-host-key-pin");
    expect(result.phase).toBe("native-link-unverified");
    expect(canSendInputFromLanAttempt(result)).toBe(false);
    expect(await coordinator.disconnect()).toEqual({ phase: "idle" });
  });

  it("keeps mDNS hints non-authoritative even if native open resolves", async () => {
    const coordinator = new LanAttemptCoordinator(nativePort());
    const result = await coordinator.open({
      kind: "discovered", host: { id: "host-a", displayName: "PC", ipv4: "172.16.0.5", port: 5443 }
    });
    expect(result.phase).toBe("native-link-unverified");
    expect(canSendInputFromLanAttempt(result)).toBe(false);
  });

  it("prohibits parallel opens and isolates attempt IDs across restarts", async () => {
    const ids: number[] = [];
    const closed: number[] = [];
    const coordinator = new LanAttemptCoordinator(nativePort(async id => { ids.push(id); }, async id => {
      closed.push(id);
    }));
    const first = await coordinator.open(intent);
    expect(first.phase).toBe("native-link-unverified");
    const duplicate = await coordinator.open(qrIntent);
    expect(duplicate).toBe(first);
    expect(ids).toHaveLength(1);
    await coordinator.disconnect();
    expect(await coordinator.open(qrIntent)).toMatchObject({ phase: "native-link-unverified", attemptId: 2 });
    expect(ids).toEqual([1, 2]);
    expect(closed).toEqual([1]);
  });

  it("invalidates in-flight open synchronously before close and rejects stale completion", async () => {
    const gate: { resume?: () => void; signal?: AbortSignal } = {};
    const pending = new Promise<void>(resolve => { gate.resume = resolve; });
    const closed: number[] = [];
    const coordinator = new LanAttemptCoordinator(nativePort(async (_id, _candidate, currentSignal) => {
      gate.signal = currentSignal;
      await pending;
    }, async id => { closed.push(id); }));
    const opening = coordinator.open(intent);
    expect(coordinator.state.phase).toBe("opening");
    const closing = coordinator.disconnect();
    expect(coordinator.state.phase).toBe("closing");
    expect(gate.signal?.aborted).toBe(true);
    // Abort is advisory: the first close cannot prove a still-pending native
    // open will not create resources afterward. Keep the attempt closing.
    expect(await closing).toEqual({ phase: "closing" });
    expect((await coordinator.open(qrIntent)).phase).toBe("closing");
    gate.resume?.();
    expect((await opening).phase).toBe("idle");
    expect(coordinator.state.phase).toBe("idle");
    expect(closed).toEqual([1, 1]);
  });

  it("latches failure when a cancelled late open cannot be closed again", async () => {
    const gate: { resume?: () => void } = {};
    const pending = new Promise<void>(resolve => { gate.resume = resolve; });
    let closeCount = 0;
    const coordinator = new LanAttemptCoordinator(nativePort(
      async () => { await pending; },
      async () => {
        closeCount++;
        if (closeCount === 2) throw new Error("late resource cleanup failed");
      },
    ));
    const opening = coordinator.open(intent);
    expect(await coordinator.disconnect()).toEqual({ phase: "closing" });
    gate.resume?.();
    expect(await opening).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(closeCount).toBe(2);
    expect(await coordinator.open(qrIntent)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(canSendInputFromLanAttempt(coordinator.state)).toBe(false);
  });

  it("serializes late-open cleanup behind a still-running first close", async () => {
    const gate: { open?: () => void; close?: () => void } = {};
    const pendingOpen = new Promise<void>(resolve => { gate.open = resolve; });
    const pendingClose = new Promise<void>(resolve => { gate.close = resolve; });
    const closeCalls: number[] = [];
    const coordinator = new LanAttemptCoordinator(nativePort(
      async () => { await pendingOpen; },
      async id => {
        closeCalls.push(id);
        if (closeCalls.length === 1) await pendingClose;
      },
    ));
    const opening = coordinator.open(intent);
    const disconnecting = coordinator.disconnect();
    await Promise.resolve();
    expect(closeCalls).toEqual([1]);
    gate.open?.();
    await Promise.resolve();
    expect(closeCalls).toEqual([1]); // no overlapping native close
    gate.close?.();
    expect((await disconnecting).phase).toBe("closing");
    expect((await opening).phase).toBe("idle");
    expect(closeCalls).toEqual([1, 1]);
  });

  it("keeps input denied through every diagnostics phase, including forged observations", async () => {
    const states: LanAttemptState[] = [
      { phase: "idle" },
      { phase: "opening", attemptId: 7, candidate: { kind: "manual-ip", ipv4: "10.1.2.3", port: 3333, trust: "unverified" } },
      { phase: "native-link-unverified", attemptId: 7, candidate: { kind: "manual-ip", ipv4: "10.1.2.3", port: 3333, trust: "unverified" } },
      { phase: "closing" },
      { phase: "blocked", reason: "cleanup-failed" },
    ];
    expect(states.every(state => canSendInputFromLanAttempt(state) === false)).toBe(true);
  });

  it("closes native resources and blocks new attempts after cleanup failure", async () => {
    const coordinator = new LanAttemptCoordinator(nativePort(async () => undefined, async () => {
      throw new Error("native close failed");
    }));
    expect((await coordinator.open(intent)).phase).toBe("native-link-unverified");
    expect(await coordinator.disconnect()).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(await coordinator.open(qrIntent)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
  });

  it("closes partially opened resources on failed native open", async () => {
    const closed: number[] = [];
    const coordinator = new LanAttemptCoordinator(nativePort(async () => {
      throw new Error("DTLS unavailable");
    }, async id => { closed.push(id); }));
    expect(await coordinator.open(intent)).toEqual({ phase: "blocked", reason: "native-transport-failed" });
    expect(closed).toEqual([1]);
    expect(canSendInputFromLanAttempt(coordinator.state)).toBe(false);
  });

  it("does not start a new native attempt while failed-open cleanup is in progress", async () => {
    const gate: { completeClose?: () => void } = {};
    const closePending = new Promise<void>(resolve => { gate.completeClose = resolve; });
    let opens = 0;
    const coordinator = new LanAttemptCoordinator(nativePort(async () => {
      opens++;
      throw new Error("fail");
    }, async () => { await closePending; }));
    const first = coordinator.open(intent);
    // Await a microtask to enter native close.
    await Promise.resolve();
    expect(coordinator.state.phase).toBe("closing");
    const concurrent = await coordinator.open(qrIntent);
    expect(concurrent.phase).toBe("closing");
    expect(opens).toBe(1);
    gate.completeClose?.();
    expect((await first).phase).toBe("blocked");
  });

  it("does not re-open after native open and native cleanup both fail", async () => {
    let opens = 0;
    const coordinator = new LanAttemptCoordinator(nativePort(async () => {
      opens++;
      throw new Error("open failed");
    }, async () => { throw new Error("close failed"); }));
    expect(await coordinator.open(intent)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(await coordinator.open(intent)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(opens).toBe(1);
  });

  it("disconnect is idempotent without an active session", async () => {
    const coordinator = new LanAttemptCoordinator();
    expect(await coordinator.disconnect()).toEqual({ phase: "idle" });
    expect(await coordinator.disconnect()).toEqual({ phase: "idle" });
  });
});
