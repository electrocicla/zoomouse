# Zoomouse — execution control room

Owner: Zoomouse M1 parent orchestrator. Canonical repo: `github.com/electrocicla/zoomouse`; branch: `main`; baseline `efac24a0997714a98df1d1bd48d0ecdba69a0d6f`.

This directory is the durable coordination record: [PLAN](PLAN.md), [TASKS](TASKS.md), [WORKLOG](WORKLOG.md), per-worker append-only summaries in `workers/<task-id>.md`. Product references: `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/QA-2026-10-30.md`.

## Definition of done and operating discipline

- Before changing code inspect neighboring files, call graph, tests, contracts, branch/head and Agents Global active claims.
- **Strict TypeScript/no `any`**, SRP/SOLID, minimal scoped diffs. Use Rust ownership/Result typing, no panics at external input boundaries. Fail closed on unapproved input, capture or networking.
- One worker owns each nonoverlapping file set. Coordinate contract changes and package/lockfiles with parent first. Parent owns `workplan/PLAN.md`, `TASKS.md`, `WORKLOG.md`, `README.md`; each worker owns one `workplan/workers/<task-id>.md`. No worker should edit another worker's file.
- Every phase must log dates/evidence to Agents Global and update the on-disk worker worklog, including SHA, commands, outcomes, blockers and exact files. Parent reconciles TASKS and global WORKLOG after verified child handoffs.
- Workers **commit and push** each completed task to the authorized branch through Git FIFO, and supply verified remote SHA; no force push, no invented CI/E2E claims. A blocked task is not DONE and may not have a commit.
- Shared source validation: `pnpm check`. Native work requires platform-specific tests/build. Physical LAN Android↔Windows consent/input is a *separate* milestone gate; M0 preview is simulated.
- Do not claim a deployment succeeded from queued/prepared status: require sent prompt, correct connector, authenticated child Agents Global connection and real task progress.
- Status vocabulary: `BACKLOG`, `READY`, `ACTIVE`, `BLOCKED`, `IN_REVIEW`, `DONE`. Update exactly one authoritative task row per phase, with evidence.
