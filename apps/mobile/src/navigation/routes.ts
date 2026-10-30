/** Navigation is a local shell until pairing and host approval are implemented. */
export type Route = "devices" | "manual-ip" | "qr" | "approval" | "controls" | "settings";

export const navigationTitle = (route: Route): string => {
  switch (route) {
    case "devices": return "Dispositivos";
    case "manual-ip": return "Conexión manual";
    case "qr": return "Código QR";
    case "approval": return "Aprobación";
    case "controls": return "Control remoto";
    case "settings": return "Ajustes";
  }
};
