/** A narrow native boundary: no transport implementation is installed in M1-B. */
export type RemoteInput =
  | Readonly<{ kind: "pointer-move"; dx: number; dy: number }>
  | Readonly<{ kind: "pointer-button"; button: "left" | "right"; pressed: boolean }>
  | Readonly<{ kind: "scroll"; dx: number; dy: number }>
  | Readonly<{ kind: "key"; code: string; pressed: boolean }>
  | Readonly<{ kind: "text"; text: string }>;

export type LanEndpoint =
  | Readonly<{ kind: "discovered"; deviceId: string }>
  | Readonly<{ kind: "manual-ip"; ipv4: string; port: number }>
  | Readonly<{ kind: "qr"; signedPayload: string }>;

export interface DiscoveredHost {
  readonly id: string;
  readonly displayName: string;
  readonly ipv4: string;
  readonly port: number;
}

export interface PendingHostApproval {
  readonly kind: "pending-host-approval";
  readonly requestId: string;
  readonly hostFingerprint: string;
}

export interface HostApprovedSession {
  readonly hostFingerprint: string;
  readonly grantId: string;
  readonly sessionId: string;
  send(input: RemoteInput): Promise<void>;
  disconnect(): Promise<void>;
}

export interface LanClient {
  discover(signal: AbortSignal): Promise<readonly DiscoveredHost[]>;
  requestHostApproval(endpoint: LanEndpoint, signal: AbortSignal): Promise<PendingHostApproval>;
  awaitApprovedSession(requestId: string, signal: AbortSignal): Promise<HostApprovedSession>;
}

export class TransportNotImplementedError extends Error {
  constructor() {
    super("LAN pairing and host authorization are not implemented in M1-B.");
    this.name = "TransportNotImplementedError";
  }
}

/** A fail-closed default: no devices, approval or trusted session can be manufactured. */
export const createUnavailableLanClient = (): LanClient => ({
  discover: async () => { throw new TransportNotImplementedError(); },
  requestHostApproval: async () => { throw new TransportNotImplementedError(); },
  awaitApprovedSession: async () => { throw new TransportNotImplementedError(); },
});
