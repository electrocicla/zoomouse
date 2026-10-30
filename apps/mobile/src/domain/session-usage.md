# Typed LAN transport boundary (M1-B)

The React Native shell deliberately does **not** instantiate a network transport. `createUnavailableLanClient()` rejects all discovery/pairing calls; no session grant can be constructed from local navigation. Buttons for remote input, QR camera and Windows consent are disabled until M1-D/E integrate a real host-provided session.

In this shell, `isInputAuthorized()` always returns false, including for a forged JavaScript `phase: "authorized"` object. Its previous phase-only test was insufficient to distinguish authentic native host consent; a UI state is never a trust boundary.

For M1-D integration: implement `LanClient` with authenticated host discovery, IP/QR validation, a bounded pairing request and per-host identity checks. The `HostApprovedSession` must be constructed only after verified Windows-side consent and an authenticated channel, then injected into the session state. Reject stale identity/grant/session; disconnect releases held input. Never re-enable input from local UI state alone.
