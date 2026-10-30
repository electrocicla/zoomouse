# Security & privacy — required controls

- Interactive Windows consent before accepting input or initiating screen capture; device/session scoped grants; local emergency stop.
- Per-device cryptographic identity, authenticated channel keys, replay protection, revocation, no permanent approval by default.
- Streaming only from explicitly approved screen/window; visible tray indicator; no covert capture or recording.
- Never enable system-level UAC secure desktop control, arbitrary shell, file operations or silent privilege escalation.
- Default firewall private/LAN only. Bind/discover on approved interfaces; avoid unauthenticated exposed TCP/UDP controls.
- No secrets in Git, APK, logs or app preview; protect runtime credentials with operating system storage.
- Clipboard/gyroscope/accounts optional and OFF by default if privacy-sensitive.
- Internet release requires independent threat model, authenticated signaling, ephemeral TURN creds, rate limits, revocation and CGNAT testing.
