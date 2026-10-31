# R9-B — Windows 11 Tauri2 native EXE source/build readiness

- Task `deployment-child-68f499b3b84e79d854aeb84eeb1b53b9`; parent `deployment-child-47eb49e2a8085a1dc072ba657b1bede7`; owner-selected connector **MK059**.
- Authenticated Agents Global session/lifecycleEpoch `ff0f170f-d21c-4963-8211-9a99f547aa24`; workspace **rootIndex13 Zoomouse**. Verified `github.com/electrocicla/zoomouse`, branch `main`, startup HEAD `6daa2afa10a67a5e2286f214d7a5e44c8fac25c4`, `origin/main` ahead=0/behind=0.
- Exclusive owned paths: `apps/windows/**` and NEW `workplan/workers/R9-B-WINDOWS.md`. No host, protocol, media, mobile, or parent shared docs/root manifest/config edits. Existing DIRTY `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md` belong to other lifecycles, never stage, clean or overwrite.

## Read-only dependency and threat-boundary audit

- `apps/windows/src-tauri/tauri.conf.json`: Tauri 2, `bundle.active=false`; `build.beforeBuildCommand="pnpm build"`, `frontendDist="../dist"`; one `main` settings window. No installer configuration is enabled.
- `apps/windows/src-tauri/Cargo.toml`: explicit Rust `[[bin]] name="zoomouse-desktop"`, `src/main.rs`, uses Tauri 2 tray icon; no Windows elevation or service declaration. A successful **owner-authorized** Tauri raw build would be expected to create `apps/windows/src-tauri/target/release/zoomouse-desktop.exe` on the default Windows MSVC x64 host. This is a *predicted path*, NOT verified file existence.
- `src-tauri/src/lib.rs`: `HostSession<DisabledInput>` and immutable `missingVerifiedChannelAndNativeApproval` gate, exactly `host_status` + `emergency_stop` Tauri commands; tray Revoke/Quit fail-closed cleanup diagnostics. `localUserSessionOnly=false` because no OS-observed interactive desktop. `host-status.ts` rejects forged Approved/Pending/input/discovery/capture/session claims and malformed IPC. No native Windows consent callback, Win32 injection, DTLS/WebRTC exporter, listening LAN host or WGC video.
- Capabilities `default.json` remain `core:default` only, scoped to `main`. Neither a renderer status nor a human-facing button is trusted device approval.
- Cross-repo dependencies: `crates/host/**` host security source (R8 receipt), `packages/protocol/**` authenticated channel fixtures, `apps/mobile/**` separate Android debug APK worker, parent root lockfile/manifest modified but excluded from this worker. No cross-ownership code mutation.

## Scoped R9-B changes

1. NEW `apps/windows/build-readiness.test.mjs`: no-build defensive regression checks for Tauri2 `bundle.active=false`, frontend build contract, explicit Cargo desktop bin, core-only capabilities, a single main window and the disabled native authority surface. Static assertions are **not** native compile or runtime proof.
2. `apps/windows/package.json`: extend existing standalone Node native-status test runner to include build-readiness tests; no dependency change or lockfile mutation.
3. `apps/windows/README.md`: add exact owner-run preflight, scoped source QA, future `tauri build --no-bundle` command and conditional EXE path; distinguish raw unsigned development EXE from MSI/NSIS installer. Future owner-approved per-user NSIS would set `installMode=currentUser`; DO NOT toggle `bundle.active` here.
4. This worker record, kept separate from parent `workplan/{PLAN,TASKS,WORKLOG,README}.md` and foreign dirty `R6-WIN.md`.

## Verified source-only QA (2026-10-31, Windows 11 root13)

Earlier focused terminal submissions were **rejected before execution** by active
Android, parent and host file/workspace claims (four attempts at 20:34–20:41Z).
These were reservation denials, **not failed tests**. After coordinating
lease release, the following seven independent tracked commands **actually ran**
in host terminal `agent-term-1793478865696-18` and all completed with exit **0**
against the shared working tree after host commit `9d05e27dba14`:

| Scope | Command | Exit / result | Host tracked runId |
|---|---|---|---|
| Node | `corepack pnpm --filter @zoomouse/windows test` | **PASS** 8/8: 4 new build contract + 4 fail-closed IPC | `termrun_8e3686be1574468c9c7f83ee9bff634e` |
| Strict TypeScript | `corepack pnpm --filter @zoomouse/windows typecheck` | **PASS**, `tsc --noEmit` | `termrun_555783e21f1049c2aca27279473cacd0` |
| ESLint | `corepack pnpm exec eslint apps/windows/src` | **PASS** | `termrun_61612d3598024dad8ac7c44b3772bcec` |
| Rust fmt | `cargo fmt --manifest-path apps/windows/src-tauri/Cargo.toml -- --check` | **PASS** | `termrun_717254575f294bc68659c35c26538c91` |
| Rust check | `cargo check --manifest-path apps/windows/src-tauri/Cargo.toml` | **PASS**, zoomouse-host/desktop | `termrun_085d889cd6c1417d99ace0af8a7eed62` |
| Rust test | `cargo test --manifest-path apps/windows/src-tauri/Cargo.toml` | **PASS**, 2 shell lib unit tests; no main/doc tests | `termrun_e92dbe0ba2164da1afaeb3f891f7492c` |
| Rust Clippy | `cargo clippy --manifest-path apps/windows/src-tauri/Cargo.toml --all-targets -- -D warnings` | **PASS** | `termrun_f86e0c651fa2433e9ff7eb3892db31dc` |

Each PASS is an authoritative host-completed exit 0 with bounded tracked
output; tests observe a live shared worktree, not a cryptographically frozen
snapshot. The existing `src-tauri/target/release/` path was checked read-only
and returned `path_not_found`, hence **no raw EXE artifact exists or is
verified**. No Vite/native release build, `tauri build`, `cargo build`,
installer, `assembleDebug`, signing, install, permission change or
A01–A17 physical test was executed. The Tauri binary still has
`DisabledInput`; no native Win32 interactive consent, DTLS-admitted
transport, functioning LAN mouse/keyboard or video can be inferred.

**Git FIFO safety:** Shared staging contained foreign Android files plus
our newly added files; parent published its doc-only commit
`3be7aa988d85` with the specialized `notcode_git_commit(files=...)`
scoped operation and observed foreign index entries preserved. R9-C host
source was published as `9d05e27dba14`, Android source as
`0b7216c7e2ef`; all were read-only dependencies, untouched here.
For R9-B, stage/commit **only** `apps/windows/build-readiness.test.mjs`,
`apps/windows/package.json`, `apps/windows/README.md` and this
`workplan/workers/R9-B-WINDOWS.md` through the same explicit-files
Git FIFO, verify authored path set then push and check origin/main 0/0.
The separate authoritative commit/push receipt must be observed; do not
infer publication from this source-validation record.

## Owner-only raw EXE handoff (UNEXECUTED)

Prerequisites: interactive Windows 11 x64, Rust `stable-x86_64-pc-windows-msvc` / Cargo, MSVC C++ build tools + Windows SDK, Node 22+, Corepack pnpm 10.33.0, Tauri2 CLI, Edge WebView2 runtime, root dependency install based on the owner-reconciled frozen lockfile. From repository root **only after owner approval and prereqs**:

```powershell
corepack pnpm install --frozen-lockfile
corepack pnpm --filter @zoomouse/windows exec tauri build --no-bundle
Test-Path .\apps\windows\src-tauri\target\release\zoomouse-desktop.exe
```

A positive `Test-Path` after an exit-0 native build would establish the *file* exists, not its launchability, correct runtime DLLs, interactive-user scope, acceptance, or remote feature functionality. There is **no installable MSI/NSIS** with current `bundle.active=false`. Installer target, per-user NSIS, signing/distribution and WebView2 dependency packaging are separate owner approvals. Locally run unsigned development EXEs do not need Microsoft Store signing. **A01–A17 physical tests: NOT RUN.** No genuine OS-local approval/DTLS adaptor, mouse/keyboard dispatch, remote capture/video or secure LAN session has been demonstrated.
