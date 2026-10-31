# Zoomouse M2-C — pure viewport geometry foundation
2026-10-30 | task `deployment-child-a72b2445a25c3235d9564d6042475ab6` | new lifecycle `f0a90bc4-2b01-4835-ae3d-177d6fd6ad86`

## Verified starting point and ownership
- RootIndex 13 `Zoomouse`, canonical `github.com/electrocicla/zoomouse`, branch `main`, HEAD `ab11b160faf0`, tracking `origin/main` ahead=0 behind=0 before M2-C edits.
- OWN ONLY `crates/viewport/**` and this file. Numerous uncommitted peer Android/Windows/control/root/parent-plan changes and M2-B docs are preserved. No shared edits.
- Read `workplan/{README,PLAN,TASKS,WORKLOG,ACCEPTANCE-M1}.md`, `docs/{ARCHITECTURE,ROADMAP,QA-2026-10-30}.md`, capture/media geometry+native contracts, protocol Zod zoom settings and Windows UI.

## High-level plan / dependency cascade (before implementation)
1. Introduce a self-contained, no-dependency Rust math library. Trusted host provides selected source ID, monitor physical bounds and topology revision, capture frame pixels, output dimensions, per-monitor DPI and verified rotation. No local discovery/measurement or native capabilities.
2. Fail closed for invalid finite numbers, zero dimensions, unsupported rotation/physical-to-capture aspect, overlapping/unversioned topology, source changes and out-of-source cursor. No automatic cross-display retargeting.
3. Compute an exact output-aspect, even crop with even offsets satisfying M2-B `CropPlan`; 1–4x zoom constrained by pixel quantization. Preserve full source for zoom OFF only when full-source aspect and chroma align; otherwise fail closed pending letterbox/native compositor agreement.
4. Independent stream, zoom and follow booleans: stream OFF emits no frame/coordinates; follow OFF freezes center, and nonmatching cursor cannot move the crop; monotonic caller-supplied elapsed milliseconds drives frame-rate-independent interpolation. Expose inverse output-to-source physical pointer geometry with half-open bounds.
5. Add deterministic negative/property-style tests, Rust fmt/test/check/clippy. Publish only our files via Git FIFO, report SHA and exact remaining native physical M2 gates.

## Threat / geometry invariants
- Geometry source tags and data are NOT a capture/session/peer grant. Host still validates active identity, source/grant generation, lock state, revocation, WGC and authenticated transport before use.
- `GetCursorPos` is already physical: never apply DPI scaling twice. Logical-to-physical conversion is separate and requires both host-provided anchors and per-monitor scale; outside the selected source fails.
- All desktop physical and frame rectangles half-open. No source leakage; reject ambiguous 90/180/270 degree pixel transforms until verified physical mapping contract. Reject topology overlaps, duplicate IDs and invalid revisions.
- M2-B requires even ROI edges/sizes, exact aspect ratio and bounded dimensions. Integer quantization implies not every requested zoom can be represented exactly; returned crop is authoritative.
- Android pointer reverse mapping requires a pinned crop/frame/output/source revision and trusted approval, not a free-floating client coordinate or a new remote selection.
- No unsafe code, OS APIs, graphics authority, permission changes, H.264/WebRTC, Android decoder, claimed benchmark, packaging or physical E2E.

## Worklog / checks / publication
- Initial planning complete; implementation and validation pending.

## 2026-10-30 - implementation and verified Rust source publication
- Files: crates/viewport/{.gitignore,Cargo.toml,Cargo.lock,README.md,src/lib.rs,src/tests.rs}; no OS/authority and no edits outside own crate/worklog.
- Pure API: strict finite inputs and bounded topology/DPI; reject unsupported rotations/aspect; half-open negative desktop source mapping; even exact-aspect crop 1-4x; independent streaming/zoom/follow; freeze center, monotonic time-based smoothing; reversible output-to-physical coordinate mapping; source revision pin.
- QA cargo fmt --manifest-path crates/viewport/Cargo.toml -- --check: PASS exit 0 (termrun_d85a99b6a88549498bcdc8a57b014663).
- QA cargo test --manifest-path crates/viewport/Cargo.toml: PASS exit 0; 13 unit tests + 0 doc tests (termrun_494b499f7ad340c584c53de3cd25fd4a).
- QA cargo check --manifest-path crates/viewport/Cargo.toml: PASS exit 0 (termrun_f3fd10f55aba4e50ae063522991c0093).
- QA cargo clippy --manifest-path crates/viewport/Cargo.toml --all-targets -- -D warnings: PASS exit 0 (termrun_22b18fdc810f46d5adc6bd1fe707e8fe); initial Clippy had 4 is_multiple_of warnings, fixed in owned source and revalidated.
- Source commit 2763ab58021a, pushed to origin/main (remote update 0a01e8a -> 2763ab5). Worklog publication follows in independent path-scoped commit.
- Peer Android M1-B 435c7ded + log 37ef9d74, Windows M1-C febd71a + log 0a01e8a: all published separately. Existing root and crates/control/M2-B dirty were preserved, never staged.
- Remaining: integrate trusted WGC source/grant generation, physical DPI/rotation measurements and display topology, real GPU even crop and H264/transport, Android reverse mapping/decode; M1 A01-A17 real-device gates and M2 physical video, performance, revocation and device-loss E2E NOT RUN / NOT ACCEPTED.
- Any geometry result is data only, never evidence of capture, authorization or input readiness.

## Independent R8 receipts recovery (2026-10-30)
- Current `main` history independently includes source commit `2763ab58021ae01b011f2ab5e6fde1cd791e048a` as an ancestor (not merely a worker assertion). Inspected current `crates/viewport/{Cargo.toml,README.md,src/lib.rs,src/tests.rs}`: 13 Rust unit test declarations, pure finite crop/geometry transforms, and no network/video/input/consent authority. Earlier source publication and original test/run IDs above remain historical receipts, not fresh QA.
- Fresh scoped Cargo checks were **not executed** at this audit checkpoint: host terminal reservation rejected on an active R8-PROTOCOL file claim, before any command or exit receipt. Fresh fmt/check/test/clippy remains pending FIFO availability. There is no physical DPI/rotation or real WGC/GPU/H.264/WebRTC/Android test evidence; M2 and A01–A17 remain NOT RUN / NOT ACCEPTED.

