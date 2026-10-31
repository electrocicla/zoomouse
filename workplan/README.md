# Zoomouse — execution control room

Owner: Zoomouse M1 parent orchestrator. Canonical repo: `github.com/electrocicla/zoomouse`; branch: `main`; baseline `efac24a0997714a98df1d1bd48d0ecdba69a0d6f`.

This directory is the durable coordination record: [PLAN](PLAN.md), [TASKS](TASKS.md), [WORKLOG](WORKLOG.md), [M1 physical acceptance matrix](ACCEPTANCE-M1.md), per-worker append-only summaries in `workers/<task-id>.md`. Product references: `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/QA-2026-10-30.md`.

## Definition of done and operating discipline

- Before changing code inspect neighboring files, call graph, tests, contracts, branch/head and Agents Global active claims.
- **Strict TypeScript/no `any`**, SRP/SOLID, minimal scoped diffs. Use Rust ownership/Result typing, no panics at external input boundaries. Fail closed on unapproved input, capture or networking.
- One worker owns each nonoverlapping file set. Coordinate contract changes and package/lockfiles with parent first. Parent owns `workplan/PLAN.md`, `TASKS.md`, `WORKLOG.md`, `README.md`; each worker owns one `workplan/workers/<task-id>.md`. No worker should edit another worker's file.
- Every phase must log dates/evidence to Agents Global and update the on-disk worker worklog, including SHA, commands, outcomes, blockers and exact files. Parent reconciles TASKS and global WORKLOG after verified child handoffs.
- Workers **commit and push** each completed task to the authorized branch through Git FIFO, and supply verified remote SHA; no force push, no invented CI/E2E claims. A blocked task is not DONE and may not have a commit.
- Shared source validation: `pnpm check`. Native work requires platform-specific tests/build. Physical LAN Android↔Windows consent/input is a *separate* milestone gate; M0 preview is simulated.
- Do not claim a deployment succeeded from queued/prepared status: require sent prompt, correct connector, authenticated child Agents Global connection and real task progress.
- Status vocabulary: `BACKLOG`, `READY`, `ACTIVE`, `BLOCKED`, `IN_REVIEW`, `DONE`. Update exactly one authoritative task row per phase, with evidence.

## R9 release-readiness tracking (2026-10-31)

The first *compilable* artifacts and the first *secure usable* LAN implementation are **different gates**. See the R9-A/B build prerequisites/commands in [PLAN](PLAN.md), the R9-A–E workstream IDs in [TASKS](TASKS.md), and timestamped facts in [WORKLOG](WORKLOG.md). A Windows `zoomouse-desktop.exe` at `apps/windows/src-tauri/target/release/` is an interactive-user shell until host trust and LAN input are implemented; `bundle.active=false` produces no installer. Android `app-debug.apk` is not accepted until an actual owner-authorized `:app:assembleDebug` succeeds with verified official wrapper and Android SDK. Release signing is not a prerequisite to local unsigned EXE debug testing.

**Ownership:** R9 orchestrator only `workplan/{PLAN,TASKS,WORKLOG,README}.md`; R9-A child `apps/mobile/**` + `workers/R9-A-ANDROID.md`; R9-B child `apps/windows/**` + `workers/R9-B-WINDOWS.md`. Four inherited dirty files (`.gitignore`, `package.json`, `pnpm-lock.yaml`, `workers/R6-WIN.md`) are preserved, neither staged nor overwritten without proven ownership. Authenticated R9-C source-only negative QA owns `crates/host/**` + `workers/R9-C-HOST-QA.md`; future production host/control/media integration requires new disjoint claims. Root `pnpm check` is not native CI; no EXE, APK, LAN, consent, input, video, or physical A01–A17 result may be inferred from it.
