# M1-E — trusted Windows input executor source foundation

- Task: `deployment-child-84d4978c2f7d086b095215765c87737e` / instance `web-child-84d4978c2f7d086b095215765c87737e`
- Parent: `zoomouse-hourly-20261030-orchestrator-r2-explicit`
- Authorized workspace: rootIndex 13 `Zoomouse`, `github.com/electrocicla/zoomouse`, branch `main`
- Observed base: `a99583f5c437`, `origin/main` ahead 0 / behind 0 at task startup.
- Status: standalone source authored. **No production transport/native input available**, no HostSession integration; validation and scoped Git publication must be confirmed separately.
- Ownership: NEW `crates/control/**` and this NEW worker log only. Never touched `crates/host/**`, `apps/windows/**`, `apps/mobile/**`, `packages/protocol/**`, root configs, or parent/peer logs.

## Cascade and design

Reviewed `docs/ARCHITECTURE.md`, `workplan/{PLAN,TASKS,ACCEPTANCE-M1}.md`, `crates/host/src/{input,boundary,consent,error,lib}.rs`, `packages/protocol/src/{index,session-policy,pairing-wire}.ts`. Existing host `HostSession<B: InputBackend>` is the **sole authorization boundary**: only it checks `AuthenticatedPeer`, real `DesktopContext`, current local consent and lease; existing default backend is disabled. Pairing hello, QR, identity strings and protocol Zod types do not confer authorization.

**Implemented files:** `crates/control/Cargo.toml`, `src/lib.rs`, `src/policy.rs`, `src/executor.rs`, `src/native.rs`, `README.md` (plus crate-local `Cargo.lock` if Cargo generates it). Pure policy checks exact session ID, UTF-8 encoded size declared by **trusted** frame decoder <=4096, monotonic sequence starting 1 (advance <=64), <=5s age, <=2s future tolerance, 90 events/60 moves/5 text per 1s, 32 held keys, finite bounded move and scroll, <=2048 UTF-16-unit text, duplicate down/up conflicts. Private executor commits state only after sink output, revokes on native failure and attempts held-state cleanup; cleanup errors latch termination and retain held-state for retry. `Drop` attempts final release. Windows SendInput wrapper verifies active console session, nonzero/non-Session0 process session, input and current thread `Default` desktop and exact dispatched INPUT count; emits Unicode through `KEYEVENTF_UNICODE`. All OS injection remains **private/unexported**, never reachable through network/JSON or public crate API.

**Explicit remaining production trust gates**: M1-D-LIVE establishes real WebRTC DataChannel with verified DTLS fingerprint/exporter and per-peer session binding. M1-C/parent must implement reviewed `HostSession<InputBackend>` adapter that calls this crate only after local consent and OS checks for *each* frame. A pure preview must never be accepted as a HostSession grant; native APIs must stay inaccessible to untrusted IPC. Reject unproved physical key mapping (KeyA/DigitN/Meta etc.), secure desktop/UAC/Session0, unavailable/locked desktop; no shell, file, process, clipboard, Internet listener or elevation. Windows desktop transitions race the OS injection API and require lock/suspend event handling; UIPI and SendInput failure can prevent cleanup, so never promise guaranteed key release if native cleanup fails. Capture/video and network encryption remain outside this source slice.

## Validation ledger (authoritative only)

- `cargo fmt --manifest-path crates/control/Cargo.toml -- --check`: initial FAIL exit1 before formatting (`termrun_15be7461c2e44841b576c65baececaa7`); after fix, **PASS** exit0 `termrun_5a0f3711e90c47f9aefd0300d5239117`.
- `cargo fmt --manifest-path crates/control/Cargo.toml`: **PASS** exit0 `termrun_3249bd8073a64a21bf3be3b627cff2d5` and again after Windows import fix `termrun_0a620754c618491ca2b7ce2fdacd97f1`.
- `cargo check --manifest-path crates/control/Cargo.toml`: initial FAIL exit101 `termrun_c97b0f5f403f4e109a97c4cdfdf962d0` due E0432 import resolution (StationsAndDesktops and RemoteDesktop modules); corrected **only owned** `Cargo.toml` and `src/native.rs`. Final **PASS** exit0 `termrun_a9136707dbfd4d188c9829085de76ee1`, Windows PowerShell using `windows-sys 0.61.2`.
- `cargo test --manifest-path crates/control/Cargo.toml`: **PASS** exit0 `termrun_8acec0ba473e471189c673029a16d366`, 7/7 unit tests, 0 doc-tests, Windows PowerShell.
- `cargo clippy --manifest-path crates/control/Cargo.toml --all-targets -- -D warnings`: **PASS** exit0 `termrun_8308176a0ace427abf659abf283e6faf`.
- All tests are isolated native compilation/unit coverage, **NOT** physical Win11 interactive desktop/SendInput execution. Windows desktop switch and UAC/UIPI are not E2E validated.
- No native sendinput executed, no Win11 physical desktop verification, no Android APK/install, no usable encrypted DataChannel; **M1 acceptance A01–A17 ALL NOT RUN**.

## Publication / coordination

- Scope Git stage/commit/push only this crate and `workplan/workers/M1-E.md`; verify SHA, `origin/main` 0/0, and reported tool receipts. Record exact result here after QA.
- Notified parent about isolated source-only boundary and informed M1-D-LIVE/M2-B of shared terminal FIFO. Released idle own file claims before their QA operations.
- Commit SHA: **PENDING**. Push receipt: **PENDING**. Parent integration acceptance: **PENDING**.

## 2026-10-30 — M1-E recovery and source publication

- Recovery child: `deployment-child-9d4424009a7aa817fbc6376961ae30a1`, instance `web-child-9d4424009a7aa817fbc6376961ae30a1`, lifecycle `49558398-9740-4b3a-bc2f-8e64b8139703`; parent `zoomouse-hourly-20261030-2216-orch`. Verified authorized rootIndex 13 `Zoomouse`, canonical `github.com/electrocicla/zoomouse`, branch `main`, initial HEAD `2763ab58021a`, origin/main ahead0/behind0; publication base after parent docs commit `25ac4392893a`.
- Preserved predecessor's entire 7-file untracked `crates/control/**` foundation without reset or rewrite; independently inspected `docs/ARCHITECTURE.md`, workplan README/PLAN/TASKS/WORKLOG/ACCEPTANCE-M1, `crates/host/src/{input,boundary,consent,error,lib}.rs`, and crate-local source, lockfile and tests. Existing private native path remains UNEXPORTED and inactive behind future HostSession/DTLS/local-consent boundaries.
- Scoped hardening in `crates/control/src/policy.rs`: reject zero-length `encoded_bytes` (no trusted frame decoder receipt); added regression assertion. This is a pure policy guard and does not authorize OS injection. No host/Android/Windows/protocol/root config edits.
- Revalidated with governed Windows PowerShell tracked runs, all exit 0 **after the scoped hardening**:
  - `cargo fmt --manifest-path crates/control/Cargo.toml -- --check` — PASS `termrun_9187efc4a9e6489fa6ae80dded7c3b18`.
  - `cargo check --manifest-path crates/control/Cargo.toml` — PASS `termrun_5da7241c59a2438c9b612acb5ecb1f34`.
  - `cargo test --manifest-path crates/control/Cargo.toml` — PASS 7/7 unit tests, 0 doc tests `termrun_46544e1fb26f4e0b908acf824b7a6260`.
  - `cargo clippy --manifest-path crates/control/Cargo.toml --all-targets -- -D warnings` — PASS `termrun_7c8eda6907dd468384db281935a7c908`.
- Coordinated terminal workspace contention with parent R5, M1-D-LIVE2 and Android M1-B-NATIVE through Agents Global. Two initially denied workspace QA attempts by parent file/Git claim, then host `admission.rs` claim, were **not executed**; subsequent tracked four commands succeeded without bypass. During scoped Git commit an unrelated Android write was rejected by the workspace claim; Android worker notified to reconcile its own edit after FIFO release. Foreign dirty/untracked files preserved.
- Scoped SOURCE commit `bd8c870b052e` (`feat(control): recover fail-closed isolated Windows input executor`), pushed `origin/main` from `25ac439` to `bd8c870` by governed Git; subsequent Git summary verified branch `main`, ahead 0 / behind 0, all seven `crates/control/**` paths clean. Source commit files: `Cargo.toml`, `Cargo.lock`, `README.md`, `src/lib.rs`, `src/policy.rs`, `src/executor.rs`, `src/native.rs` under `crates/control/`.
- **Gates still open:** no real authenticated WebRTC/DTLS DataChannel, no trusted host adapter to private native sink, no real Win11 SendInput emission, secure desktop/UAC behavior or partial cleanup physical validation, no Android APK/Windows installer, no device LAN tests. A01–A17 remain **NOT RUN**; M1-E is SOURCE-ONLY, not M1 acceptance. Worker-log commit/push receipt recorded through Agents Global and reported to the parent after publication.
