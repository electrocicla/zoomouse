# Zoomouse M1-C-QA — Tauri 2 Windows source/build integration

- **Task:** deployment-child-7d93f6ee2b706c428f7505c7abbae424 (new child lifecycle; parent zoomouse-hourly-20261030-1819).
- **Repo / scope:** github.com/electrocicla/zoomouse, `main`, authorized MK059 Zoomouse rootIndex13, connected Agents Global epoch `5501dabf-eb0a-4335-9b70-d13232acdf50`.
- **Starting HEAD:** `121429f1b97b67a6a1aed6c696933868b0190961`, tracking origin/main ahead0/behind0 before peer publications. Existing M1-C Windows scaffold was already published in `d333dfe32f1a`. No initial `apps/windows/**` dirty files.
- **Exclusive ownership:** `apps/windows/**` and this NEW worker log only. Parent controls root package/lockfile and top-level workplan; peers control Android, Rust host and protocol.
- **Plan:** read root plan/tasks/acceptance and architecture/QA; inspect Rust host public API plus Tauri JS/Rust commands and consumers; harden denial/revocation/status reporting; run local frontend/native checks; publish path-scoped only via authorized Git FIFO.

## Scoped changes / security review

1. Added `src/host-status.ts` strict runtime validator for `invoke<unknown>("host_status")`. Reject malformed/inconsistent claims of approved remote input, including stale/cleanup-failed/non-user-session states. No new command can approve a remote device.
2. `src/App.tsx` treats missing/invalid native state as **Unverified**, never fabricated Denied/interactive-user confirmation; clearly distinguishes failed emergency-stop acknowledgment from confirmed cleanup, invalidates older in-flight status responses on revoke, refreshes when a hidden tray window refocuses, and explains local approval is **not implemented**. Emergency stop remains user-accessible.
3. `src-tauri/src/lib.rs` tray Revoke now surfaces the settings window on cleanup failure instead of silently swallowing it. Window-close hides to tray; only explicit tray Quit attempts native cleanup and exits when successful. No listener, elevation, Session 0, remote approval, input injection, capture or shell commands were added.
4. `host-status.test.mjs` and per-app `test` script exercise 3 node:test cases using Node 22's experimental TS strip-types; no new dependency or root lockfile edits. Source SVG icon plus generated Windows ICO/PNG/Square icons and app-local Cargo.lock support the successful native check. Generated mobile/iOS/macOS icons and Tauri JSON schemas are excluded by app-local ignore files; no installer built.
5. React `19.3.0` / react-dom `19.3.1` peer mismatch remains **parent-owned** root lockfile/dependency coordination; app dependency ranges were not changed here.

## Command receipts (2026-10-30 UTC)

| Gate | Exact command | Tracked run | Result |
|---|---|---|---|
| Baseline Windows TS | `pnpm --filter @zoomouse/windows typecheck` | `termrun_908500856baf4f389769cc2b5fc48219` | exit 0, before source edits |
| Runtime status tests | `pnpm --filter @zoomouse/windows test` | `termrun_0f8cbc7e18b54903b64ff744b13d180f` | **PASS** exit 0, 3/3 assertions groups |
| Post-change strict TypeScript | `pnpm --filter @zoomouse/windows typecheck` | `termrun_0425b1a1755e459085b77e5d53f715a6` | **PASS** exit 0 |
| Post-change frontend production bundle | `pnpm --filter @zoomouse/windows build` | `termrun_4399cb7712344c9c85ebf419a0749285` | **PASS** exit 0, Vite 7.3.7, 32 modules |
| Native icons | `pnpm --filter @zoomouse/windows exec tauri icon src-tauri/icon.svg` | `termrun_0fc471d409494bf19e36e243ba1042aa` | **PASS** exit 0, Windows ICO/PNG/Square logos created (one earlier attempt rejected before execution) |
| Native Tauri | `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_e747b22cd3a3496d8bbe2fe30214ac26` | **PASS** exit 0, Windows toolchain Tauri 2.12.5 / zoomouse-host check (2m17s) |

Other attempted Windows test/build commands were rejected **before execution** while M1-D-WIRE/M1-B held separate workspace/file claims; these are coordination blockers, not failures. M1-C source claims were explicitly released after edits so peers could run validation. Never bypass the Agents Global workspace lease or terminal operation.

## Release and physical gates

- `bundle.active=false`; **NO installer/MSI, signed binary or reproducible package is claimed**. Frontend Vite output is not Tauri native compile.
- A01–A17 physical LAN/device tests remain **NOT RUN**; no Android↔Win11 authenticated pairing, DTLS channel, Win32 input, live Windows capture/video or secure desktop verification.
- Rust M1-D Ed25519 pairing substrate and TypeScript M1-D-WIRE contracts are parallel owned changes, not equivalent to a remotely approved active Tauri session.
- Git stage/commit/push: **PENDING** scoped Git FIFO approval. Never stage peer/parent dirty paths. Parent must reconcile completion after authoritative push receipt.

Native Rust check is verified; publication receipt pending. Manual Windows UI launch, signed installer and physical pairing remain unverified.

## 2026-10-30 recovery by deployment-child-548f354e81071946b69b92892356e241

- **Authorized root/branch:** Zoomouse rootIndex13, github.com/electrocicla/zoomouse main, observed HEAD ab11b160faf0, origin/main ahead0/behind0 at admission. Fresh Agents Global epoch 9ae13db5-74ec-49f0-851e-6c42d2c88881. Former M1-C lifecycle was retired; its previously modified/untracked files, native icons and Cargo.lock were preserved in place, not recreated/reset.
- **Plan/cascade:** reviewed workplan README/PLAN/TASKS/WORKLOG/ACCEPTANCE-M1, architecture/roadmap/M0 QA, full former M1-C-QA notes, app-local Tauri/React source, and crates/host HostSession/DisabledInput public trust boundary; source cannot grant approval, open network listener, capture, or inject Win32 input.
- **Scoped corrections:** Native host_status previously advertised localUserSessionOnly=true without OS-observed DesktopContext. The M1-C scaffold now advertises false until trusted Windows session detection is wired. Renderer parser rejects inconsistent Approved/active capabilities under unknown session or cleanup failure. Added negative Node test coverage and documented unverified session scope in apps/windows/README.md. All edits confined to apps/windows/**.
- **FIFO incidents (no command execution):** first guarded native write rejected before mutation by Android M1-B tracked reservation, then retried safely after a free slot and completed; first node test command was rejected before execution by M1-B file claims, and second rejected before execution by M2-C crates/viewport/Cargo.toml claim. Both peers were notified. All four M1-C source-file claims explicitly released to unblock Android. No tests/builds or Git mutations were claimed from these denials.
- **Post-recovery validation, authoritative tracked terminal runs (2026-10-30 UTC):**

  | Command | Run ID | Result |
  |---|---|---|
  | `pnpm --filter @zoomouse/windows test` | `termrun_bbaed8b28d5a480088f807d67a9c6a08` | PASS exit 0, **4/4 Node cases** (including unknown desktop/cleanup failure negatives) |
  | `pnpm --filter @zoomouse/windows typecheck` | `termrun_af02a73cc7b045558c4136c2743eeee4` | PASS exit 0 |
  | `pnpm --filter @zoomouse/windows build` | `termrun_1aff039dcf7640e1be94a0408dcce69c` | PASS exit 0, Vite 7.3.7 / 32 modules |
  | `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check` | `termrun_ffb660bf10154e39b0c1f98bbf1af452` | Initial FAIL exit 1: three formatting diffs in owned `lib.rs`, subsequently corrected with guarded write |
  | Same `cargo fmt --check` | `termrun_902f563922bf4543b991a08b2ffe14c0` | PASS exit 0 after correction |
  | `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_a5be7afa084647118bf9a634c9351945` | PASS exit 0, compiled `zoomouse-host` + `zoomouse-desktop` |
  | `cargo clippy --manifest-path apps/windows/src-tauri/Cargo.toml --all-targets -- -D warnings` | `termrun_d44830cf5e514e95bae881e607d39627` | PASS exit 0 |
  | `cargo test --manifest-path apps/windows/src-tauri/Cargo.toml` | `termrun_cc907151c2034c9191e47a3216236b0e` | PASS exit 0, native compile & runner; **0 app unit tests** plus 0 doctests (not a real behavior test) |

- **Source Git publication:** explicitly staged exactly **26 changed/untracked regular files within `apps/windows/**`**, including app README/manifest, Tauri main host/tray source, renderer + status parser + 4 Node tests, local `Cargo.lock`, SVG/native Windows icons and ignore manifests. Verified **zero foreign staged paths**. Commit `febd71a88dec` (`feat(windows): recover fail-closed Tauri host shell and native QA`) pushed to `origin/main`, push receipt from `37ef9d7` to `febd71a`, post-push Git **ahead 0 / behind 0**; all `apps/windows/**` paths clean. This separate M1-C-QA log is to be published in a docs-only commit.
- **Remaining acceptance gates:** Physical A01–A17 remain NOT RUN. No native MSI/installer/package, Windows UI launch/manual tray walkthrough, Android↔Win11 authenticated pairing or remote input, OS-attested interactive DesktopContext, WGC video, WebRTC/DataChannel or performance benchmark was executed or claimed. `bundle.active=false`. Parent/root `package.json`, `pnpm-lock.yaml`, shared workplan and peers' files preserved untouched.
