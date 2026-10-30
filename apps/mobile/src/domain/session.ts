import type { HostApprovedSession, PendingHostApproval } from "./transport";

export type ConnectionState =
  | Readonly<{ phase: "offline" }>
  | Readonly<{ phase: "discovering" }>
  | Readonly<{ phase: "awaiting-host-approval"; request: PendingHostApproval }>
  | Readonly<{ phase: "authorized"; session: HostApprovedSession }>
  | Readonly<{ phase: "failed"; explanation: string }>;

export const initialConnectionState: ConnectionState = { phase: "offline" };

/**
 * M1-B contains NO authenticated transport or native input adapter. A JS state
 * discriminator cannot prove host consent, grant freshness or channel identity.
 * Deny every input attempt until M1-D/E supply an independently validated
 * host-authenticated transport/session gate. Windows must enforce consent too.
 */
export const isInputAuthorized: (state: ConnectionState) => boolean = () => false;

export const connectionLabel = (state: ConnectionState): string => {
  switch (state.phase) {
    case "offline": return "Sin conexión";
    case "discovering": return "Buscando equipos";
    case "awaiting-host-approval": return "Pendiente de aprobación en Windows";
    case "authorized": return "Sesión aprobada (sin transporte validado)";
    case "failed": return "Conexión no disponible";
  }
};
