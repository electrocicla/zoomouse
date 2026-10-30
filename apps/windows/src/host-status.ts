/**
 * Renderer boundary for the local, read-only Tauri status command.
 * Unknown IPC payloads must never be interpreted as an approved session.
 */
export type ConsentPhase = "Denied" | "Pending" | "Approved" | "Revoked";

export interface HostStatus {
  readonly phase: ConsentPhase;
  readonly inputEnabled: boolean;
  readonly cleanupFailed: boolean;
  readonly discoveryEnabled: boolean;
  readonly captureEnabled: boolean;
  readonly localUserSessionOnly: boolean;
}

function isConsentPhase(value: unknown): value is ConsentPhase {
  return value === "Denied" || value === "Pending" || value === "Approved" || value === "Revoked";
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
    typeof fields.localUserSessionOnly !== "boolean"
  ) {
    return null;
  }

  // Unknown/failed local session state cannot imply approval or active capabilities.
  if (
    (fields.phase === "Approved" && (fields.cleanupFailed || !fields.localUserSessionOnly)) ||
    (fields.inputEnabled && fields.phase !== "Approved") ||
    ((!fields.localUserSessionOnly || fields.cleanupFailed) &&
      (fields.inputEnabled || fields.discoveryEnabled || fields.captureEnabled))
  ) {
    return null;
  }

  return {
    phase: fields.phase,
    inputEnabled: fields.inputEnabled,
    cleanupFailed: fields.cleanupFailed,
    discoveryEnabled: fields.discoveryEnabled,
    captureEnabled: fields.captureEnabled,
    localUserSessionOnly: fields.localUserSessionOnly
  };
}
