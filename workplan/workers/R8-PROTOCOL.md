# R8-PROTOCOL — source-only channel proof fixture foundation

**Date:** 2026-10-30 CLST / 2026-10-31 UTC.  
**Agent:** R8-PROTOCOL, `deployment-child-c852e955eda7c24a2e70035c74eae1ed`; instance `web-child-c852e955eda7c24a2e70035c74eae1ed`.  
**Parent:** `zoomouse-r8-orch-20261030-2120`.  
**Verified repository:** `github.com/electrocicla/zoomouse`, branch `main`, initial HEAD `f698f3420ee2`, `origin/main` ahead 0/behind 0.

## Objective and high-level plan

1. Verify authorized Zoomouse rootIndex13 and identity; inspect all M1 workplan/security docs, Rust R7 channel proof contract, TS protocol tests and Android native transport placeholders; preserve all foreign work.
2. Implement only the grounded native transcript *preimage* and bounded deterministic **test-only** JSON fixtures, not a guessed production TS↔Rust wire/verifier.
3. Exercise byte ordering, Rust SHA-256 fixture, tampering of every observed channel field, malformed/unknown/duplicate/noncanonical/oversized JSON, single-use attempts and no-authority invariants.
4. Run strict TypeScript, lint and Vitest; publish ONLY this worker's paths after evidence-backed validation.

## Files owned and changed

- `packages/protocol/src/channel-proof-fixture.ts` — strict canonical fixture parser + serializer, deterministic 245-byte Rust R7 preimage, one-shot fixture inspection with `nativeAuthority: false`.
- `packages/protocol/src/channel-proof-fixture.test.ts` — 5 defensive fixture tests incl. synthetic SHA-256 transcript digest `ed8e3b2a9850e4cd0d462b326a5b6f104d37a7b10d8e247832a866f44d645ea0`.
- `packages/protocol/package.json` — adds the new isolated `./channel-proof-fixture` subpath export.
- `packages/protocol/CHANNEL-PROOF.md` — exact native field ordering, boundaries, negatives, missing production wire/crypto integration.
- `workplan/workers/R8-PROTOCOL.md` — this scoped evidence log.

## Decisions and cascading impact

- Rust `crates/host/src/channel_binding.rs` provides SHA-256 domain+field ordering and 20-second one-use Ed25519 proof but has **no serialized native proof frame, established DTLS exporter producer, independently trusted Android counterpart or authoritative host-local Windows approval**. Accordingly NO production TS verification API was invented.
- The synthetic parser accepts only canonical strict ASCII JSON (<= 1024 bytes); it cannot prove any supplied key/fingerprint/exporter came from an established native channel. The one-shot guard burns invalid or repeated fixture inspections, but is not a native anti-replay authority.
- No modification to `apps/**`, `crates/**`, CI, shared workplan, root config or other workers. Multiple foreign dirty paths and concurrent worker edits were preserved. Each peer/parent workspace-claim collision was handled through Agents Global; own file claims released when completed.

## Authoritative checks

| Command | Host validation receipt | Exit | Evidence |
|---|---|---:|---|
| `pnpm typecheck` | `validation_37849609395c4f5f`, run `run_8fe63c3479744d5d` | 0 | Protocol TypeScript and preview TypeScript PASS |
| `pnpm lint` | `validation_b2c2758d0f9b4446`, run `run_5400b3f5f2c6401a` | 0 | Scoped protocol+preview ESLint PASS |
| `pnpm test:run` | `validation_ada9c863ae8c4ecf`, run `run_40ee90fe811141a5` | 0 | Protocol **35/35** (5 new) and preview **7/7** PASS |

Earlier typecheck `validation_0ded3cc959ee4d7d` exited 2 (missing Node typings and overnarrowed test literal), fixed in test-only code; earlier lint `validation_990f8205d3c144c6` exited 1 (unused destructured names), fixed. Both subsequently PASS above. These checks are source-only, not native Android, Rust integration, WebRTC, Windows packaging, or physical acceptance.

## Remaining blockers and physical gates

**R8-PROTOCOL SOURCE-ONLY; NOT M1 ACCEPTED.** Pending: reviewed native WebRTC DTLS/SCTP producer, Rust↔Android authenticated proof wire contract and signature acceptance vectors, OS-local Windows consent, trusted host admission/input gate, native build/install and A01–A17 physical tests (all **NOT RUN**). Production bootstrap remains `DisabledMissingNativeAuthority`; no remote input/video grant follows from TS fixture parsing. Publication SHA and upstream receipt are attached to the final Agents Global log and parent evidence.
