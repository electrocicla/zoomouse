import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const GRADLE_BIN_SHA256 = "2ab2958f2a1e51120c326cad6f385153bb11ee93b3c216c5fccebfdfbb7ec6cb";
export const GRADLE_WRAPPER_SHA256 = "55243ef57851f12b070ad14f7f5bb8302daceeebc5bce5ece5fa6edb23e1145c";
const DISTRIBUTION_URL = "https\\://services.gradle.org/distributions/gradle-9.4.1-bin.zip";

function nonemptyFile(file) {
  try {
    const info = statSync(file);
    return info.isFile() && info.size > 0;
  } catch {
    return false;
  }
}

function textFile(file) {
  try { return readFileSync(file, "utf8"); }
  catch { return null; }
}

function property(fileContent, key) {
  return fileContent?.split(/\r?\n/).find((line) => line.startsWith(`${key}=`))?.slice(key.length + 1).trim() ?? null;
}

function normalized(dir) {
  return path.resolve(dir).replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
}

export function inspectAndroidReadiness({
  mobileDir = path.dirname(fileURLToPath(import.meta.url)),
  env = process.env,
  nodeVersion = process.versions.node,
} = {}) {
  const checks = [];
  const check = (id, ok, action) => checks.push({ id, ok: Boolean(ok), action });
  const version = nodeVersion.split(".").map(Number);
  check("node", version.length === 3 && version.every(Number.isInteger) &&
    (version[0] > 22 || (version[0] === 22 && version[1] >= 13)),
    "Install Node.js >=22.13.0.");

  const android = path.join(mobileDir, "android");
  const wrapper = path.join(android, "gradle", "wrapper");
  const props = textFile(path.join(wrapper, "gradle-wrapper.properties"));
  check("wrapper-distribution", property(props, "distributionUrl") === DISTRIBUTION_URL,
    "Use Gradle's official 9.4.1 binary distribution.");
  check("wrapper-distribution-sha", property(props, "distributionSha256Sum") === GRADLE_BIN_SHA256,
    "Pin the published Gradle 9.4.1 bin ZIP SHA-256.");
  for (const file of ["gradlew", "gradlew.bat"]) {
    check(file, nonemptyFile(path.join(android, file)), `Generate the official ${file} wrapper script.`);
  }
  const jar = path.join(wrapper, "gradle-wrapper.jar");
  const digest = nonemptyFile(jar) ? createHash("sha256").update(readFileSync(jar)).digest("hex") : null;
  check("official-wrapper-jar", digest === GRADLE_WRAPPER_SHA256,
    "Generate and verify the official Gradle 9.4.1 wrapper JAR (do not fabricate or use an unverified binary).");

  const javaHome = env.JAVA_HOME?.trim();
  const release = javaHome ? textFile(path.join(javaHome, "release")) : null;
  const major = /(?:^|\r?\n)JAVA_VERSION="(\d+)(?:\.[^"]*)?"/.exec(release ?? "");
  check("jdk-release", major !== null && Number(major[1]) >= 17,
    "Set JAVA_HOME to JDK >=17; JDK 21 recommended (release metadata required).");
  check("jdk-tools", Boolean(javaHome && ["java", "javac"].every((name) =>
    ["", ".exe"].some((extension) => nonemptyFile(path.join(javaHome, "bin", name + extension))))),
  "JAVA_HOME must include java and javac.");

  const home = env.ANDROID_HOME?.trim();
  const root = env.ANDROID_SDK_ROOT?.trim();
  const conflicting = Boolean(home && root && normalized(home) !== normalized(root));
  const sdk = conflicting ? null : home || root;
  check("sdk-root", Boolean(sdk), "Set ANDROID_HOME to the Android SDK; ANDROID_SDK_ROOT must agree if defined.");
  check("sdk-platform-37", Boolean(sdk && nonemptyFile(path.join(sdk, "platforms", "android-37", "android.jar"))),
    "Install Android SDK Platform 37.");
  check("sdk-build-tools-37", Boolean(sdk && ["aapt2", "aapt2.exe"].some((name) =>
    nonemptyFile(path.join(sdk, "build-tools", "37.0.0", name)))),
  "Install Android SDK Build-Tools 37.0.0.");
  const ndk = sdk ? textFile(path.join(sdk, "ndk", "28.2.13676358", "source.properties")) : null;
  check("ndk-28", Boolean(ndk && /^Pkg\.Revision\s*=\s*28\.2\.13676358\s*$/m.test(ndk)),
    "Install Android NDK 28.2.13676358.");
  check("rn-gradle-plugin", existsSync(path.join(mobileDir, "node_modules", "@react-native", "gradle-plugin")),
    "Install pnpm workspace dependencies after parent-owned lockfile integration.");

  return { readyForBuildAttempt: checks.every(({ ok }) => ok), checks };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = inspectAndroidReadiness();
  for (const { id, ok, action } of result.checks) {
    process.stdout.write(`${ok ? "PASS" : "BLOCK"} ${id}${ok ? "" : ` — ${action}`}\n`);
  }
  process.stdout.write(result.readyForBuildAttempt
    ? "Static prerequisites present; compilation, debug APK, and device acceptance remain UNVERIFIED.\n"
    : "Missing Android native prerequisites; no build was attempted.\n");
  if (!result.readyForBuildAttempt) process.exitCode = 1;
}
