# R10-ORCH-MOBILE-RACE — native LAN cancellation race closure

**Repository:** `github.com/electrocicla/zoomouse`, branch `main`. **Base:** `a693c7fdee82` (R10 planning published); selected MK059 Zoomouse rootIndex13 observed afresh. **Owner:** current R10 orchestrator, no delegated child. **Scope:** `apps/mobile/src/domain/lan-attempt.ts`, `apps/mobile/src/domain/lan-attempt.test.ts`, this NEW note, parent `workplan/{TASKS,WORKLOG}.md` only. No root lockfile/foreign dirty files touched.

## Root cause and cascade
- `LanAttemptCoordinator.disconnect()` previously called `native.close(id)` and immediately reported `idle` even when `native.open(id,...,AbortSignal)` was still pending. Abort is advisory; a native adapter may ignore it or finish resource creation after the first close. The stale open continuation suppressed JS state resurrection but never closed newly created native resources. This could leave a native connection/resource alive and permit an overlapping attempt before cleanup. The existing `NativeSecureLanPort` is unavailable by default, so this is a future native-integration safety defect, **not evidence of a shipping socket**.
- Consumers: `apps/mobile/src/domain/lan-attempt.ts` coordinator, `lan-attempt.test.ts` asynchronous native mock, `session.ts` deny-all session policy, `packages/protocol/CHANNEL-PROOF.md` non-authoritative fixture, Rust host admission and Tauri fail-closed state. No new native authority, permission, transport or input API introduced.
- The coordinator now tracks a pending native open, serializes the immediate close with a second close after a cancelled open eventually settles, and retains `closing` until that late cleanup completes. New attempts cannot start during the unresolved cancellation. Any close failure permanently latches `blocked/cleanup-failed`; `canSendInputFromLanAttempt` remains always false.
- Added two adversarial cases: delayed native open after immediate close requires a second close, and late cleanup failure latches denial; concurrent first/late closes are serialized. Existing disconnect test updated to reflect truthful `closing` state.

## Current-run validation receipts (Windows PowerShell via governed Not-Code terminal)
- `corepack pnpm --filter @zoomouse/mobile test`: **exit 0**, four Node Android readiness tests and **98/98 Vitest** (5 files), tracked `termrun_c7193c0928d24d80b337eeb048d10ad2`, 2026-11-01 04:17:58–04:18:02 UTC.
- `corepack pnpm --filter @zoomouse/mobile typecheck`: **exit 0**, tracked `termrun_c68ca6e068e24485b77600d99eec502d`, 04:18:12–04:18:16 UTC.
- `corepack pnpm --filter @zoomouse/mobile lint`: **exit 0**, tracked `termrun_7dfaaeaf38e44af8b0f593314634503b`, 04:18:23–04:18:28 UTC.
- These are host-tracked PowerShell completion receipts for live workspace source, not immutable release builds. Android `:app:assembleDebug`, Win11 EXE/installer, real DTLS exporter/cert proof, OS-local approval, input/video and physical A01–A17 were **NOT RUN**.

## Coordination and remaining gates
- R10-WIN child deployment **refused before Send**: parent Agents Global identity ambiguous (implicit host-generated task, no explicit host-supplied `agentInstanceId`). No child created. Do not fabricate an instance identity or replay Send.
- Four inherited dirty paths `.gitignore`, `package.json`, `pnpm-lock.yaml`, `workplan/workers/R6-WIN.md` preserved. R10-WIN/HOST/ANDROID native integrations remain READY/BLOCKED as applicable; this race fix is independent source hardening.
- Commit/push evidence is recorded in the parent worklog after authoritative Git publication, not assumed here.
