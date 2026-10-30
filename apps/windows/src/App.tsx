import { invoke, isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

type ConsentPhase = "Denied" | "Pending" | "Approved" | "Revoked";

interface HostStatus {
  readonly phase: ConsentPhase;
  readonly inputEnabled: boolean;
  readonly cleanupFailed: boolean;
  readonly discoveryEnabled: boolean;
  readonly captureEnabled: boolean;
  readonly localUserSessionOnly: boolean;
}

const noNativeStatus: HostStatus = {
  phase: "Denied",
  inputEnabled: false,
  cleanupFailed: false,
  discoveryEnabled: false,
  captureEnabled: false,
  localUserSessionOnly: true
};

export function App() {
  const [status, setStatus] = useState<HostStatus>(noNativeStatus);
  const [error, setError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const nativeApp = isTauri();

  const refresh = useCallback(async () => {
    if (!nativeApp) {
      setStatus(noNativeStatus);
      setError("Native Windows host is not running. Web preview cannot control a PC.");
      return;
    }
    try {
      const next = await invoke<HostStatus>("host_status");
      setStatus(next);
      setError(null);
    } catch {
      setStatus(noNativeStatus);
      setError("Cannot confirm native host state. All remote control remains disabled.");
    }
  }, [nativeApp]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const emergencyStop = async () => {
    if (!nativeApp || stopping) return;
    setStopping(true);
    try {
      await invoke("emergency_stop");
      await refresh();
    } catch {
      setStatus(noNativeStatus);
      setError("Emergency-stop acknowledgement failed. Verify the host has stopped.");
    } finally {
      setStopping(false);
    }
  };

  return (
    <main className="shell">
      <header>
        <span className="eyebrow">ZOOMOUSE / WINDOWS 11</span>
        <h1>Host controls</h1>
        <p>Private LAN foundation. Pairing and native input are not yet enabled.</p>
      </header>
      <section className="card" aria-labelledby="status-title">
        <div className="row">
          <h2 id="status-title">Session status</h2>
          <span className="badge">{status.phase}</span>
        </div>
        <p className="muted">
          No remote device can control your keyboard or mouse until a future
          authenticated pairing flow receives explicit approval here.
        </p>
        <dl>
          <div><dt>Remote input</dt><dd>{status.inputEnabled ? "Enabled" : "Disabled"}</dd></div>
          <div><dt>LAN discovery</dt><dd>{status.discoveryEnabled ? "Enabled" : "Not configured"}</dd></div>
          <div><dt>Screen streaming</dt><dd>{status.captureEnabled ? "Enabled" : "Disabled"}</dd></div>
          <div><dt>Session scope</dt><dd>{status.localUserSessionOnly ? "Interactive user only" : "Unverified"}</dd></div>
        </dl>
      </section>
      <section className="card" aria-labelledby="safety-title">
        <h2 id="safety-title">Local control</h2>
        <p className="muted">Revocation attempts native key/button cleanup even when no device is connected.</p>
        {status.cleanupFailed && (
          <p className="error" role="alert">Native input cleanup failed. Verify keys and buttons locally before reconnecting.</p>
        )}
        <button type="button" onClick={() => void emergencyStop()} disabled={!nativeApp || stopping}>
          {stopping ? "Revoking…" : "Emergency stop / revoke all"}
        </button>
        <button className="secondary" type="button" onClick={() => void refresh()}>
          Refresh status
        </button>
        {error && <p role="alert" className="error">{error}</p>}
      </section>
      <footer>
        M1-C scaffold only: no listening port, input injection, screen capture,
        service installation or administrator privileges.
      </footer>
    </main>
  );
}
