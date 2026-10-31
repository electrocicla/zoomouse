# R9-C — Host trust/receipt negative QA

**Child:** `deployment-child-61ac7dda0a105c94aa5179ceff15b959` · **Parent:** `deployment-child-47eb49e2a8085a1dc072ba657b1bede7` · **Connector:** MK059 · **Root:** 13 Zoomouse · **Branch:** main.

## Observed baseline and ownership

- Verified canonical `github.com/electrocicla/zoomouse`, baseline HEAD `6daa2afa10a67a5e2286f214d7a5e44c8fac25c4`, origin/main ahead0 behind0. Parent subsequently published separate workplan commit `3be7aa988d85` with origin/main 0/0.
- Source edits exclusively `crates/host/src/{consent,boundary,admission,channel_binding}.rs`; this new worker file `workplan/workers/R9-C-HOST-QA.md`. Apps, control, protocol, root config, shared workplan, R6-WIN and pre-existing dirty/untracked paths are foreign and untouched. In particular, no production/native bootstrap or Tauri hooks.
- Read R7-HOST-CHANNEL and R8-HOST-TRUST worker receipts, actual host Rust code and active collaboration/claims. R7 proof is a crate-private SHA-256/Ed25519 challenge over asserted backend observations with 20s monotonic TTL, not a live DTLS exporter. R8 host pending native callback is one-use OS-CSPRNG nonce with 60s monotonic expiry, not real Win32-proven local consent. Verified authority constructors and actual remote input remain intentionally unreachable in production.

## Threat model and scoped repair

- **Discovered genuine direct-boundary gap:** `ConsentController::authorize` previously conflated a wrong peer and a changed Windows interactive user-session ID into `HostError::WrongSession`. `HostSession::dispatch` intentionally does not revoke on wrong peer (to avoid cross-peer denial of service), but therefore did **not release potentially held keys** on a Windows user switch. Changed OS-session mismatch to `HostError::DesktopUnavailable` (peer mismatch remains `WrongSession`). Existing dispatch fail-closed handler now revokes grant and releases all native held input before returning; stale grant cannot return when original user comes back. This is **source state-machine behavior**, not a Win32 event producer.
- Strengthened boundary regression to assert one native release, zero unauthorized dispatch, phase Revoked and replay denial after OS user switches away and back.
- Added admission negatives: competing pending session cannot replace original channel/host-only nonce; OS user switch during pending callback consumes pending, revokes enrollment, releases input and rejects replay; original accepted pending callback remains usable after rejected competing request. Tests use **crate-private test fixtures**, not external OS approval.
- Added channel proof negatives: exact 20s deadline expires while 1ns-before succeeds, invalid signature consumes challenge and subsequent valid replay is refused. No receipt/authority object is minted and no public constructor exposed.

## Tracked validation receipts

All commands executed on authorized Windows PowerShell terminal `I:\\Zoomouse`, `crates/host/Cargo.toml`, with exact host-tracked completion and exit codes:

| Command | Result | Run ID |
| --- | --- | --- |
| `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` initial | EXIT1, two rustfmt-only differences in new admission tests; **not** a test pass | `termrun_2301ac52a94248388f8419ffb956a414` |
| `cargo fmt --manifest-path crates/host/Cargo.toml` | PASS EXIT0, formatted only scoped host Rust | `termrun_1049dc82d9644e27b042d13c5bef68f7` |
| `cargo fmt --manifest-path crates/host/Cargo.toml -- --check` final | PASS EXIT0 | `termrun_761c423661314186acd6fe8cccc428e4` |
| `cargo check --manifest-path crates/host/Cargo.toml` | PASS EXIT0 | `termrun_6d894e3cdc48455ca712d0494b52f268` |
| `cargo test --manifest-path crates/host/Cargo.toml` | PASS EXIT0: **60/60 unit tests, 1/1 doctest** | `termrun_d1148bd7db984b0a835846566bddf318` |
| `cargo clippy --manifest-path crates/host/Cargo.toml --all-targets -- -D warnings` | PASS EXIT0 | `termrun_60650402f3484250a6c2a1216a649ef4` |

- Workspace reservation denials were **pre-execution**, never represented as tests: first peer/parent active file and workspace claims, including Android R9-A note and parent workplan; coordinated via Agents Global critical inbox + direct messages. No bypass or concurrent lease override.
- Git summary after checks: origin/main ahead0 behind0, main HEAD `3be7aa988d85`; all four host source changes only working-tree `M` with **index blank**. Peer app/config/workers dirties are unstaged and preserved.

## Remaining blockers (do not misrepresent as release-ready)

No real Win32 trusted local callback/interactive desktop token, logout/lock/UAC monitoring, DTLS exporter and peer certificate evidence producer, authenticated Android channel, input enablement, or secure LAN listener exists. The controlled Rust tests and historical worker receipts cannot qualify A01–A17 physical acceptance, APK, .exe artifact, installer or working LAN mouse/keyboard/capture/zoom. Those gates require separately reviewed native authority and owner-run build/physical acceptance. No Windows/Android packaging, signing, installation or private/public listener was run by this child.

## Publication

Commit/push only the four owned Rust files and this note after Git FIFO and verified exclusive index; record actual published SHA in Agents Global final receipt (this note deliberately makes no unverified commit claim).
