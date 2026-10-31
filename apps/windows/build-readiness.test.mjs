import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

function appFile(path) {
  return readFileSync(new URL(`./${path}`, import.meta.url), "utf8");
}

test("Tauri native build contract is raw EXE only, not a Windows installer", () => {
  const config = JSON.parse(appFile("src-tauri/tauri.conf.json"));
  const scripts = JSON.parse(appFile("package.json")).scripts;
  assert.equal(config.$schema, "https://schema.tauri.app/config/2");
  assert.equal(config.bundle?.active, false);
  assert.equal(config.build.beforeBuildCommand, "pnpm build");
  assert.equal(config.build.frontendDist, "../dist");
  assert.equal(scripts.build, "tsc --noEmit && vite build");
  assert.equal(scripts.tauri, "tauri");
});

test("Cargo emits the named user-session Tauri executable", () => {
  const cargo = appFile("src-tauri/Cargo.toml");
  assert.match(cargo, /^name = "zoomouse-desktop"$/m);
  assert.match(
    cargo,
    /^\[\[bin\]\]\r?\nname = "zoomouse-desktop"\r?\npath = "src\/main.rs"$/m
  );
  assert.match(cargo, /^tauri = \{ version = "2", features = \["tray-icon"\] \}$/m);
  assert.match(appFile("src-tauri/src/main.rs"), /zoomouse_desktop::run\(\)/);
});

test("Tauri window capabilities remain local and minimal", () => {
  const config = JSON.parse(appFile("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(appFile("src-tauri/capabilities/default.json"));
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, ["core:default"]);
  assert.deepEqual(config.app.windows.map((window) => window.label), ["main"]);
  assert.match(config.app.security.csp, /default-src 'self'/);
  assert.doesNotMatch(config.app.security.csp, /https?:\/\/(?:\*|0\.0\.0\.0)/);
});

test("renderer IPC cannot request native approval or activate remote input", () => {
  const rust = appFile("src-tauri/src/lib.rs");
  assert.match(rust, /tauri::generate_handler!\[host_status, emergency_stop\]/);
  assert.match(rust, /LocalHostState::new\(DisabledInput\)/);
  assert.match(rust, /const APPROVAL_GATE: &str = "missingVerifiedChannelAndNativeApproval"/);
});
