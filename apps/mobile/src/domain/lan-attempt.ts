import { discoveredLanCandidate, hostQrCandidate, manualLanCandidate } from "./lan-bootstrap";
import type { LanBootstrapCandidate } from "./lan-bootstrap";

/**
 * Network discovery and QR data are untrusted hints, not transport credentials.
 * The native implementation must provide authenticated DTLS with pinned host
 * identity, native channel binding and separately verified Windows consent.
 * This JS boundary does not expose remote input or HostApprovedSession.
 */
export type LanConnectIntent =
  | Readonly<{ kind: "manual-ip"; ipv4: unknown; port: unknown }>
  | Readonly<{ kind: "discovered"; host: unknown }>
  | Readonly<{ kind: "qr"; payload: unknown }>;

export type LanAttemptState =
  | Readonly<{ phase: "idle" }>
  | Readonly<{ phase: "opening"; attemptId: number; candidate: LanBootstrapCandidate }>
  | Readonly<{ phase: "native-link-unverified"; attemptId: number; candidate: LanBootstrapCandidate }>
  | Readonly<{ phase: "closing" }>
  | Readonly<{ phase: "blocked"; reason:
    "invalid-candidate" | "native-transport-unavailable" | "native-transport-failed" | "cleanup-failed" }>;

export interface NativeSecureLanPort {
  /**
   * Must be implemented in an audited native module; never use plaintext TCP,
   * fetch or an unpinned TLS client. Resolving only reports native link setup,
   * NOT host-local approval, grant freshness or permission to send input.
   */
  open(attemptId: number, candidate: LanBootstrapCandidate, signal: AbortSignal): Promise<void>;
  /** Idempotently tear down this exact attempt and release native resources. */
  close(attemptId: number): Promise<void>;
}

export class NativeLanUnavailableError extends Error {
  constructor() {
    super("No audited Android DTLS/host-identity native adapter is installed.");
    this.name = "NativeLanUnavailableError";
  }
}

/** No sockets, permissions or native module are installed in this build. */
export function createUnavailableNativeLanPort(): NativeSecureLanPort {
  return {
    open: async () => { throw new NativeLanUnavailableError(); },
    close: async () => undefined,
  };
}

function candidateFor(intent: LanConnectIntent): LanBootstrapCandidate | null {
  switch (intent.kind) {
    case "manual-ip":
      return manualLanCandidate(intent.ipv4, intent.port);
    case "discovered":
      return discoveredLanCandidate(intent.host);
    case "qr":
      return hostQrCandidate(intent.payload);
  }
  return null; // Reject unexpected runtime discriminator from a JS/native boundary.
}

type ActiveAttempt = Readonly<{ id: number; abort: AbortController }>;

/**
 * Single-flight, cancellation-aware native transport seam. There is no
 * transition to "authorized" here: even a forged native completion remains
 * observational. A failed native cleanup keeps the coordinator blocked.
 */
export class LanAttemptCoordinator {
  private stateValue: LanAttemptState = { phase: "idle" };
  private active: ActiveAttempt | null = null;
  private nextId = 0;
  private cleanupFailed = false;
  private pendingOpenId: number | null = null;
  private disconnectClose: Promise<void> | null = null;

  /** A cancelled native open may settle after the first close. Close again
   * before allowing a new attempt, so late native resources cannot survive. */
  private async finishCancelledOpen(attemptId: number): Promise<void> {
    try {
      await this.disconnectClose;
      await this.native.close(attemptId);
      if (!this.cleanupFailed && this.stateValue.phase === "closing") {
        this.stateValue = { phase: "idle" };
      }
    } catch {
      this.cleanupFailed = true;
      this.stateValue = { phase: "blocked", reason: "cleanup-failed" };
    }
  }

  constructor(private readonly native: NativeSecureLanPort = createUnavailableNativeLanPort()) {}

  get state(): LanAttemptState { return this.stateValue; }

  async open(intent: LanConnectIntent): Promise<LanAttemptState> {
    if (this.cleanupFailed) return this.stateValue;
    if (this.active !== null || this.stateValue.phase === "closing") return this.stateValue;

    const candidate = candidateFor(intent);
    if (candidate === null) {
      this.stateValue = { phase: "blocked", reason: "invalid-candidate" };
      return this.stateValue;
    }

    const attempt: ActiveAttempt = { id: ++this.nextId, abort: new AbortController() };
    this.active = attempt;
    this.pendingOpenId = attempt.id;
    this.stateValue = { phase: "opening", attemptId: attempt.id, candidate };

    try {
      await this.native.open(attempt.id, candidate, attempt.abort.signal);
      this.pendingOpenId = null;
      if (this.active === attempt) {
        this.stateValue = { phase: "native-link-unverified", attemptId: attempt.id, candidate };
      } else {
        await this.finishCancelledOpen(attempt.id);
      }
    } catch (error: unknown) {
      this.pendingOpenId = null;
      if (this.active !== attempt) {
        await this.finishCancelledOpen(attempt.id);
        return this.stateValue;
      }
      attempt.abort.abort();
      this.active = null;
      this.stateValue = { phase: "closing" };
      try {
        await this.native.close(attempt.id);
        this.stateValue = { phase: "blocked", reason: error instanceof NativeLanUnavailableError
          ? "native-transport-unavailable" : "native-transport-failed" };
      } catch {
        this.cleanupFailed = true;
        this.stateValue = { phase: "blocked", reason: "cleanup-failed" };
      }
    }
    return this.stateValue;
  }

  /**
   * Invalidates the attempt BEFORE asynchronous native cleanup. A late
   * open result cannot resurrect a disconnected or revoked session.
   */
  async disconnect(): Promise<LanAttemptState> {
    if (this.cleanupFailed) return this.stateValue;
    if (this.stateValue.phase === "closing") return this.stateValue;
    const attempt = this.active;
    if (attempt === null) {
      this.stateValue = { phase: "idle" };
      return this.stateValue;
    }
    this.active = null;
    attempt.abort.abort();
    const wasOpening = this.pendingOpenId === attempt.id;
    this.stateValue = { phase: "closing" };
    // Serialize the immediate close with the late-open close. A native port
    // may ignore AbortSignal and finish opening after this close completes.
    const close = Promise.resolve().then(() => this.native.close(attempt.id));
    this.disconnectClose = close;
    try {
      await close;
      if (!wasOpening && this.stateValue.phase === "closing") {
        this.stateValue = { phase: "idle" };
      }
    } catch {
      this.cleanupFailed = true;
      this.stateValue = { phase: "blocked", reason: "cleanup-failed" };
    } finally {
      if (this.disconnectClose === close) this.disconnectClose = null;
    }
    return this.stateValue;
  }
}

/** JS observations and candidate pins can never authorize input. */
export function canSendInputFromLanAttempt(state: LanAttemptState): false {
  switch (state.phase) {
    case "idle":
    case "opening":
    case "native-link-unverified":
    case "closing":
    case "blocked":
      return false;
  }
}
