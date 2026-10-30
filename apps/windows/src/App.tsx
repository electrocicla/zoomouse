import { invoke, isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { parseHostStatus, type HostStatus } from "./host-status";

export function App() {
  const [status, setStatus] = useState<HostStatus | null>(null);
  const statusRequest = useRef(0);
  const [error, setError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const nativeApp = isTauri();

  const refresh = useCallback(async () => {
    const request = ++statusRequest.current;
    if (!nativeApp) {
      setStatus(null);
      setError("Native Windows host is not running. This page cannot confirm host state.");
      return;
    }
    try {
      const next = parseHostStatus(await invoke<unknown>("host_status"));
      if (!next) throw new Error("Invalid native host status");
      if (request !== statusRequest.current) return;
      setStatus(next);
      setError(null);
    } catch {
      if (request !== statusRequest.current) return;
      setStatus(null);
      setError("Cannot verify native host state. Do not assume control or cleanup has stopped.");
    }
  }, [nativeApp]);

  useEffect(() => {
    void refresh();
    // A failed tray revoke surfaces the main window; focus must re-read native cleanup.
    const onFocus = () => { void refresh(); };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refresh]);

  const emergencyStop = async () => {
    if (!nativeApp || stopping) return;
    // Invalidate any older status query before a potentially slow native revoke.
    ++statusRequest.current;
    setStatus(null);
    setStopping(true);
    try {
      await invoke("emergency_stop");
      await refresh();
    } catch {
      ++statusRequest.current;
      setStatus(null);
      setError("Emergency-stop acknowledgement failed. Host cleanup is UNVERIFIED; check locally.");
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
          <span className="badge">{status?.phase ?? "Unverified"}</span>
        </div>
        <p className="muted">
          No remote device can control your keyboard or mouse until a future
          authenticated pairing flow receives explicit local approval.
          This build has no approval or remote-input command.
        </p>
        <dl>
          <div><dt>Remote input</dt><dd>{status ? (status.inputEnabled ? "Enabled" : "Disabled") : "Unverified"}</dd></div>
          <div><dt>LAN discovery</dt><dd>{status ? (status.discoveryEnabled ? "Enabled" : "Not configured") : "Unverified"}</dd></div>
          <div><dt>Screen streaming</dt><dd>{status ? (status.captureEnabled ? "Enabled" : "Disabled") : "Unverified"}</dd></div>
          <div><dt>Session scope</dt><dd>{status?.localUserSessionOnly ? "Interactive user only" : "Unverified"}</dd></div>
        </dl>
      </section>
      <section className="card" aria-labelledby="safety-title">
        <h2 id="safety-title">Local control</h2>
        <p className="muted">Revocation attempts native key/button cleanup even when no device is connected.</p>
        {status?.cleanupFailed && (
          <p className="error" role="alert">Native input cleanup failed. Verify keys and buttons locally before reconnecting.</p>
        )}
        <button type="button" onClick={() => void emergencyStop()} disabled={!nativeApp || stopping}>
          {stopping ? "Revoking…" : "Emergency stop / revoke all"}
        </button>
        <button className="secondary" type="button" onClick={() => void refresh()} disabled={stopping}>
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
