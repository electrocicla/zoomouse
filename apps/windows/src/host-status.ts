/**
 * Renderer boundary for the local, read-only Tauri status command.
 * Unknown IPC payloads must never be interpreted as an approved session.
 */
export type ConsentPhase = "Denied" | "Pending" | "Approved" | "Revoked";
export type ReleaseOutcome = "notAttempted" | "confirmed" | "failed";
export type DesktopObservation = "interactiveDefault" | "unavailable";

/** Only valid for the current fail-closed native shell, not future approved hosts. */
export const NATIVE_APPROVAL_GATE = "missingVerifiedChannelAndNativeApproval" as const;

export interface HostStatus {
  readonly phase: ConsentPhase;
  readonly inputEnabled: boolean;
  readonly cleanupFailed: boolean;
  readonly discoveryEnabled: boolean;
  readonly captureEnabled: boolean;
  readonly localUserSessionOnly: boolean;
  readonly desktopObservation: DesktopObservation;
  readonly approvalGate: typeof NATIVE_APPROVAL_GATE;
  readonly lastReleaseOutcome: ReleaseOutcome;
}

function isConsentPhase(value: unknown): value is ConsentPhase {
  return value === "Denied" || value === "Pending" || value === "Approved" || value === "Revoked";
}

function isDesktopObservation(value: unknown): value is DesktopObservation {
  return value === "interactiveDefault" || value === "unavailable";
}

function isReleaseOutcome(value: unknown): value is ReleaseOutcome {
  return value === "notAttempted" || value === "confirmed" || value === "failed";
}

export function parseHostStatus(value: unknown): HostStatus | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }

  const fields = value as Record<string, unknown>;
  if (
    !isConsentPhase(fields.phase) ||
    typeof fields.inputEnabled !== "boolean" ||
    typeof fields.cleanupFailed !== "boolean" ||
    typeof fields.discoveryEnabled !== "boolean" ||
    typeof fields.captureEnabled !== "boolean" ||
    typeof fields.localUserSessionOnly !== "boolean" ||
    !isDesktopObservation(fields.desktopObservation) ||
    fields.approvalGate !== NATIVE_APPROVAL_GATE ||
    !isReleaseOutcome(fields.lastReleaseOutcome)
  ) {
    return null;
  }

  // The OS-observed desktop signal is diagnostic only. No native local
  // approval callback or DTLS channel exists in this app revision; reject
  // even syntactically valid "Approved" or input-enabled payloads.
  if (
    (fields.phase !== "Denied" && fields.phase !== "Revoked") ||
    fields.inputEnabled ||
    fields.discoveryEnabled ||
    fields.captureEnabled ||
    fields.localUserSessionOnly ||
    (fields.lastReleaseOutcome !== "notAttempted" && fields.phase !== "Revoked") ||
    (fields.lastReleaseOutcome === "failed" && !fields.cleanupFailed)
  ) {
    return null;
  }

  return {
    phase: fields.phase,
    inputEnabled: fields.inputEnabled,
    cleanupFailed: fields.cleanupFailed,
    discoveryEnabled: fields.discoveryEnabled,
    captureEnabled: fields.captureEnabled,
    localUserSessionOnly: fields.localUserSessionOnly,
    desktopObservation: fields.desktopObservation,
    approvalGate: fields.approvalGate,
    lastReleaseOutcome: fields.lastReleaseOutcome
  };
}
