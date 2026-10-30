# R6-WIN — Windows Tauri 2 consent/tray boundary

- **Task:** `deployment-child-2916e1d1a373a461da21cff95814a0c1`, child of `zoomouse-hourly-20261030-r6-7b42`.
- **Repo/branch:** `github.com/electrocicla/zoomouse`, `main`; authorized rootIndex **13**.
- **Verified starting HEAD:** `ac474bd1a3f0`, `origin/main` ahead 0 / behind 0.
- **Starting dirty files (not owned, never modified):** `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/M2-B.md`, `workplan/workers/M2-C.md`.
- **Ownership:** `apps/windows/**` and this log only. Host, protocol, control, mobile and parent workplan are read-only.
- **Coordination:** Agents Global session `0708c73f-c53e-47bb-b3a0-9f48bd0549b6`, parent READY messaged. Concurrent parent, Android and host workers are active with separate file ownership.

## Source audit and plan (before edits)

Read `docs/{ARCHITECTURE,ROADMAP,QA-2026-10-30,SECURITY}.md`,
`workplan/{PLAN,TASKS,ACCEPTANCE-M1}.md`, existing Windows Tauri
source/tests/capabilities and Rust admission, boundary, consent and control
interfaces. `crates/host/src/admission.rs` intentionally has **no production
constructors** for `VerifiedChannelEvidence`, `NativePairingApprovalReceipt` or
`NativeApprovalReceipt`. It keeps the native-authority bootstrap disabled.
A renderer Approve button or serialized Tauri approval command would bypass
the required trust chain, so this worker **does not** introduce one.

Plan:
1. Keep `HostSession<DisabledInput>` in a Tauri-owned mutex, and expose a
   fixed missing-authority readiness gate and last backend-release result.
2. Route visible emergency stop and tray revoke/quit to one host-owned
   release method, latch release failure and preserve fail-closed quit.
3. Refuse false/native-inconsistent IPC status claims in strict TypeScript,
   surface explicit blocked native consent/transport and release diagnostics.
4. Run scoped Node/TypeScript/Vite and Rust fmt/check/test/clippy, publish only
   verified owned paths with Git FIFO; no Tauri installer packaging.

## Scoped implementation (source-only)

- `apps/windows/src-tauri/src/lib.rs`: `LocalHostState<B>` owns the
  `HostSession<B>` and `ReleaseOutcome`; `host_status` is read-only and
  advertises fixed `missingVerifiedChannelAndNativeApproval`. Stop and tray
  use the same revocation/cleanup method; release errors latch a failed
  outcome, and Quit still refuses silent exit on failure. Rust test fixtures
  cover disabled-shell invariants and a synthetic failing release backend.
- `apps/windows/src/host-status.ts`: strict denial-only IPC schema for the
  present native shell; malformed or forged Approved/Pending/input/desktop/
  discovery/capture claims cannot be displayed as verified.
- `apps/windows/src/App.tsx`: explicit disabled approval, session scope,
  release outcome and failure handling; status re-query after failed stop.
- `apps/windows/host-status.test.mjs`: negative schema/authority and release
  outcome tests.
- `apps/windows/README.md`: exact native authority integration contract.

## Validation and publication

**Windows 11 / root 13 / current working source.** The source validations below were
run as individual host-tracked PowerShell terminal commands in
`agent-term-1793402651529-11`, with exact run IDs and authoritative completed
exit codes. They are local *source checks*, not installer, Win32-input or
physical-device acceptance. These commands ran against a live shared worktree
(`sourceSnapshotVerified=false`); peer host/mobile files were concurrently edited
but are not Windows-owned.

| Command | Outcome | Host run ID |
| --- | --- | --- |
| `pnpm --filter @zoomouse/windows test` | **PASS** exit 0; 4 Node defensive IPC tests | `termrun_2969b8a2140b479d81d5339bf9b1d23d` |
| `pnpm --filter @zoomouse/windows typecheck` | **PASS** exit 0 | `termrun_90e16ced8e45485f9841012f08f147b5` |
| `pnpm exec eslint apps/windows/src` | **PASS** exit 0; no app-specific `lint` script exists | `termrun_7f8827dc6d3e4d248c35040613bb633a` |
| `pnpm --filter @zoomouse/windows build` | **PASS** exit 0; `tsc --noEmit && vite build`; frontend bundle only | `termrun_86cae47899f745e0a5db374c9d59b9fe` |
| `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check` | initial **FAIL** exit 1; one local assertion wrapping issue, repaired within Windows ownership | `termrun_1c756711f7d5416180bb266a98a04e72` |
| same Cargo format check after source repair | **PASS** exit 0 | `termrun_6230709577e949eeb37f18d54e314c16` |
| `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` | **PASS** exit 0 | `termrun_4ad21137d24e41249e82b23bac163c6e` |
| `cargo test --manifest-path apps/windows/src-tauri/Cargo.toml` | **PASS** exit 0; 2 Rust lib tests, 0 main/doc tests | `termrun_8d6fffde088d4a5a9a46cb9c2558a704` |
| `cargo clippy --manifest-path apps/windows/src-tauri/Cargo.toml --all-targets -- -D warnings` | **PASS** exit 0 | `termrun_4813b876895f4d93a9faaf17f17ae0b6` |

**Coordination/blocked attempts:** Before the successful validations,
the first terminal test was not started because the workspace reservation was
refused due to parent `workplan/TASKS.md` claim, and a subsequent test attempt
due to Android's `apps/mobile/src/domain/lan-attempt.ts` claim. After our first
4/4 passing test run, a TypeScript check attempt was refused due to HOST's
`crates/host/src/boundary.rs` claim. These are **no-op reservation denials**,
not failed test results. The Android/host/parent were informed via Agents Global,
our first six source claims were voluntarily released to serialize validation,
and all actual checks were run using host-governed operations. No foreign claim
was removed or bypassed.

**Publication:** Commit SHA/push receipt pending below; do not infer a
published build from these passing source checks.

## Acceptance gates that remain

No native DTLS/WebRTC verified channel adapter; no OS-observed interactive
Windows session, native UI callback or production consent receipt; no real
Win32 input release integration; no live private-LAN transport, firewall or
WGC/capture; no physical A01–A17 tests; no installer or signed release.
Only a trusted native Windows callback may eventually mint device/session/
channel-bound receipts. Source tests are **not** physical consent acceptance.
