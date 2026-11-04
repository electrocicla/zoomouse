import { createUnavailableNativeLanPort, NativeLanUnavailableError } from "./lan-attempt";
import type { NativeSecureLanPort } from "./lan-attempt";

/**
 * The Kotlin module is deliberately deny-only until audited native DTLS/SCTP,
 * Ed25519 peer pinning and a matching Windows proof producer actually exist.
 * JS never receives DTLS exporter/certificate material or input authority.
 */
interface SecureLanNativeModule {
  openSecureChannel(
    attemptId: number,
    ipv4: string,
    port: number,
    pinnedHostPublicKeyHex: string | null,
  ): Promise<unknown>;
  closeAttempt(attemptId: number): Promise<unknown>;
}

function nativeModule(value: unknown): SecureLanNativeModule | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const candidate = value as Record<string, unknown>;
  if (typeof candidate.openSecureChannel !== "function" ||
      typeof candidate.closeAttempt !== "function") return null;
  return candidate as unknown as SecureLanNativeModule;
}

function unavailableRejection(error: unknown): boolean {
  if (typeof error !== "object" || error === null) return false;
  return (error as { code?: unknown }).code === "E_NATIVE_CHANNEL_UNAVAILABLE";
}

/**
 * A callable JS or native method is NOT native proof. QR keys are unverified
 * until locally enrolled and proven; neither the native link nor input can
 * become authorized through this bridge. A future reviewed implementation must
 * replace this deny-only port, not change JS state discriminators.
 */
export function createAndroidNativeLanPort(candidateModule: unknown): NativeSecureLanPort {
  const module = nativeModule(candidateModule);
  if (module === null) return createUnavailableNativeLanPort();

  return {
    open: async (attemptId, candidate, signal) => {
      if (signal.aborted || candidate.kind !== "qr-host-key-pin") {
        throw new NativeLanUnavailableError();
      }
      try {
        await module.openSecureChannel(
          attemptId, candidate.ipv4, candidate.port, candidate.hostPublicKeyHex,
        );
      } catch (error: unknown) {
        if (unavailableRejection(error)) throw new NativeLanUnavailableError();
        throw error;
      }
      // Even if a compromised or misconfigured bridge resolves, do not
      // interpret that as proof of a native, mutually authenticated channel.
      throw new NativeLanUnavailableError();
    },
    close: async attemptId => {
      // A future native backend must close the exact attempt and release
      // resources; failure is retained by LanAttemptCoordinator fail-closed.
      await module.closeAttempt(attemptId);
    },
  };
}
