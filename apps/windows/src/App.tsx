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
      // Re-read the native latch even on a failed revoke acknowledgement.
      // A tray/IPC release error must remain visible after the window opens.
      await refresh();
      setError("Emergency-stop failed or its acknowledgement was lost. Do not assume native input was released; verify locally.");
    } finally {
      setStopping(false);
    }
  };

  return (
    <main className="shell">
      <header>
        <span className="eyebrow">ZOOMOUSE / WINDOWS 11</span>
        <h1>Host controls</h1>
        <p>Private LAN foundation. Pairing, approval and native input remain disabled.</p>
      </header>
      <section className="card" aria-labelledby="status-title">
        <div className="row">
          <h2 id="status-title">Session status</h2>
          <span className="badge">{status?.phase ?? "Unverified"}</span>
        </div>
        <p className="muted">
          This installed source revision cannot grant remote control or screen access.
          The Windows host has neither authenticated DTLS channel evidence nor an
          OS-observed user-session approval callback. No renderer approval command exists.
        </p>
        <dl>
          <div><dt>Remote input</dt><dd>{status ? (status.inputEnabled ? "Enabled" : "Disabled") : "Unverified"}</dd></div>
          <div><dt>LAN discovery</dt><dd>{status ? (status.discoveryEnabled ? "Enabled" : "Not configured") : "Unverified"}</dd></div>
          <div><dt>Screen streaming</dt><dd>{status ? (status.captureEnabled ? "Enabled" : "Disabled") : "Unverified"}</dd></div>
          <div><dt>Session scope</dt><dd>{status?.localUserSessionOnly ? "Interactive user only" : "Unverified"}</dd></div>
          <div><dt>Local approval</dt><dd>{status ? "Disabled — native authority missing" : "Unverified"}</dd></div>
          <div><dt>Release outcome</dt><dd>{
            !status ? "Unverified" :
            status.lastReleaseOutcome === "confirmed" ? "Disabled-backend cleanup succeeded" :
            status.lastReleaseOutcome === "failed" ? "FAILED — local review required" :
            "Not requested"
          }</dd></div>
        </dl>
      </section>
      <section className="card" aria-labelledby="safety-title">
        <h2 id="safety-title">Local control</h2>
        <p className="muted">Revocation attempts native key/button cleanup even when no device is connected.</p>
        {status?.cleanupFailed && (
          <p className="error" role="alert">Native input cleanup failed. Verify keys and buttons locally before reconnecting. Quit is blocked when cleanup fails.</p>
        )}
        <p className="muted">
          Emergency stop revokes the host state and attempts backend release.
          A successful result here does not establish physical Win32 input testing.
          Tray Emergency stop and Quit follow the same fail-closed release path.
        </p>
        <button type="button" onClick={() => void emergencyStop()} disabled={!nativeApp || stopping}>
          {stopping ? "Revoking…" : "Emergency stop / revoke all"}
        </button>
        <button className="secondary" type="button" onClick={() => void refresh()} disabled={stopping}>
          Refresh status
        </button>
        {error && <p role="alert" className="error">{error}</p>}
      </section>
      <footer>
        R6-WIN source-only shell: no listening port, input injection, screen capture,
        service installation, administrator privileges, or consent presentation.
        To enable device/session-bound consent later, the host must provide trusted
        DTLS peer/key evidence and a native Windows interactive-session approval receipt.
        A01–A17 physical acceptance is not complete.
      </footer>
    </main>
  );
}
