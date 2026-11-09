# Zoomouse R15 — scoped task register (2026-11-09 CLST)
| ID | P | State at start | Owner / acceptance |
|---|---|---|---|
| R15-PLAN | P0 | IN PROGRESS | Parent three R15 docs only, source graph and branch/provenance gates |
| R15-WIN-RECOVERY | P0 | SOURCE PUBLISHED 5f22207572ca; NATIVE GATE BLOCKED | Own child `deployment-child-f4879f45fdf5b3c5b256d85fa86df947` recovered R14 `native.rs` + original note; 16/16 control and 9/9 Tauri +9/9 Windows TS source tests, check/clippy PASS; origin/main 0/0. No physical SendInput/approval |
| R15-HOST-NATIVE | P0 | BLOCKED: deployment outcome unknown; native authority absent | Native backend DTLS exporter/certificate/SCTP producer not present; attempted distinct child deployment `12fcf6e9-bf11-4208-ba5f-dc4fe183446f` did not achieve authenticated connect; NEVER duplicate Send without reconciliation |
| R15-ANDROID-NATIVE | P0 | PENDING native runtime/SDK | apps/mobile/**, secure Kotlin↔host link never inferred from TS |
| R15-WIN-OS | P0 | PENDING independent OS-owned consent | apps/windows/** only after Win recovery scope separated |
| R15-TOOLCHAIN | P0 | CURRENT FAIL exit1 `termrun_b2a3174e46ee437f86c769f487d6b47e` | Missing official gradlew/gradlew.bat, verified wrapper JAR, SDK root/Platform37/Build-Tools37/NDK28.2; JDK/Node/RN plugin available; no APK attempted. Confirmed compact JSON `termrun_8eb6aeb6491b456f90177727466de9c8` exit1 |
| R15-M1-A01-A17 | P0 OWNER | NOT RUN | Physically verify authenticated LAN, Win consent/input/cleanup/device lock/reconnect after native producers + artifacts exist |
| R15-M2 | P1 | UNIMPLEMENTED live | WGC+D3D11+MF H264 encrypted WebRTC, Android decode, cursor-follow tested |
| R15-M3/M4 | later | DEFERRED | UX parity and Internet/TURN after native M1/M2 gates |

R14 source published SHA Android a3881f7 / d6280cc and host 804ad33 / dce939a. R15 Windows source+note `5f22207572ca` pushed, origin/main 0/0, leaving only five original owner/R13 dirty paths. R15 parent planning docs source SHA `6e797791f4a2` pushed. Startup `main` HEAD dce939a and 7 dirty paths were the pre-recovery boundary. Do not equate stale R14/R13 ACTIVE rows with live agent state; no physical native acceptance.
