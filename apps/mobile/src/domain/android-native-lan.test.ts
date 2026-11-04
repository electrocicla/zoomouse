import { describe, expect, it } from "vitest";
import { createAndroidNativeLanPort } from "./android-native-lan";
import { canSendInputFromLanAttempt, LanAttemptCoordinator, NativeLanUnavailableError } from "./lan-attempt";

const pin = "a".repeat(64);
const qr = { kind: "qr", payload: `zoomouse://pair/v1?ip=192.168.1.10&port=50001&host_key=${pin}` } as const;
const manual = { kind: "manual-ip", ipv4: "192.168.1.10", port: 50001 } as const;

describe("Android Kotlin secure LAN bridge (deny-only until native DTLS exists)", () => {
  it("fails closed if the native module is absent or malformed", async () => {
    for (const module of [undefined, null, {}, { openSecureChannel: async () => undefined }]) {
      const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(module));
      const state = await coordinator.open(qr);
      expect(state).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
      expect(canSendInputFromLanAttempt(state)).toBe(false);
    }
  });

  it("never sends manual-IP or mDNS candidates without independently pinned host identity", async () => {
    let opens = 0;
    const native = {
      openSecureChannel: async () => { opens++; },
      closeAttempt: async () => undefined,
    };
    const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(native));
    expect(await coordinator.open(manual)).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
    expect(await coordinator.open({ kind: "discovered", host: {
      id: "pc", displayName: "PC", ipv4: "192.168.1.10", port: 50001,
    } })).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
    expect(opens).toBe(0);
  });

  it("passes only a QR host pin as an untrusted hint and never treats native rejection as a grant", async () => {
    const calls: unknown[][] = [];
    const closed: number[] = [];
    const native = {
      openSecureChannel: async (...args: unknown[]) => {
        calls.push(args);
        throw { code: "E_NATIVE_CHANNEL_UNAVAILABLE" };
      },
      closeAttempt: async (id: number) => { closed.push(id); },
    };
    const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(native));
    const result = await coordinator.open(qr);
    expect(result).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
    expect(calls).toEqual([[1, "192.168.1.10", 50001, pin]]);
    expect(closed).toEqual([1]);
    expect(canSendInputFromLanAttempt(result)).toBe(false);
  });

  it("refuses even a forged successful native bridge completion", async () => {
    let closes = 0;
    const native = {
      openSecureChannel: async () => ({ verified: true, exporter: pin, grant: "fabricated" }),
      closeAttempt: async () => { closes++; },
    };
    const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(native));
    expect(await coordinator.open(qr)).toEqual({ phase: "blocked", reason: "native-transport-unavailable" });
    expect(closes).toBe(1);
  });

  it("latches native cleanup failure and never permits a later native attempt", async () => {
    let opens = 0;
    const native = {
      openSecureChannel: async () => { opens++; throw new NativeLanUnavailableError(); },
      closeAttempt: async () => { throw new Error("close failure"); },
    };
    const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(native));
    expect(await coordinator.open(qr)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(await coordinator.open(qr)).toEqual({ phase: "blocked", reason: "cleanup-failed" });
    expect(opens).toBe(1);
  });

  it("serializes cancellation and closes a late native result twice without granting input", async () => {
    let release: (() => void) | undefined;
    const gate = new Promise<void>(resolve => { release = resolve; });
    const closed: number[] = [];
    const native = {
      openSecureChannel: async () => { await gate; },
      closeAttempt: async (attemptId: number) => { closed.push(attemptId); },
    };
    const coordinator = new LanAttemptCoordinator(createAndroidNativeLanPort(native));
    const opening = coordinator.open(qr);
    expect(coordinator.state.phase).toBe("opening");
    expect(await coordinator.disconnect()).toEqual({ phase: "closing" });
    expect((await coordinator.open(qr)).phase).toBe("closing");
    release?.();
    expect(await opening).toEqual({ phase: "idle" });
    expect(closed).toEqual([1, 1]);
    expect(canSendInputFromLanAttempt(coordinator.state)).toBe(false);
  });
});
