import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, test } from "node:test";
import { GRADLE_BIN_SHA256, inspectAndroidReadiness } from "./android-readiness.mjs";

const temporary = [];
afterEach(() => {
  for (const dir of temporary.splice(0)) rmSync(dir, { recursive: true, force: true });
});

function fixture() {
  const mobileDir = mkdtempSync(path.join(os.tmpdir(), "zoomouse-android-preflight-"));
  temporary.push(mobileDir);
  const put = (name, contents) => {
    const destination = path.join(mobileDir, name);
    mkdirSync(path.dirname(destination), { recursive: true });
    writeFileSync(destination, contents);
  };

  put("android/gradle/wrapper/gradle-wrapper.properties",
    "distributionUrl=https\\://services.gradle.org/distributions/gradle-9.4.1-bin.zip\n" +
    `distributionSha256Sum=${GRADLE_BIN_SHA256}\n`);
  put("android/gradlew", "official generated script placeholder for fixture");
  put("android/gradlew.bat", "official generated bat placeholder for fixture");
  put("android/gradle/wrapper/gradle-wrapper.jar", "not an official wrapper jar");
  put("jdk/release", 'JAVA_VERSION="21.0.6"\n');
  put("jdk/bin/java", "fixture");
  put("jdk/bin/javac", "fixture");
  put("sdk/platforms/android-37/android.jar", "fixture");
  put("sdk/build-tools/37.0.0/aapt2", "fixture");
  put("sdk/ndk/28.2.13676358/source.properties", "Pkg.Revision = 28.2.13676358\n");
  put("node_modules/@react-native/gradle-plugin/package.json", "{}");
  return {
    mobileDir,
    env: {
      JAVA_HOME: path.join(mobileDir, "jdk"),
      ANDROID_HOME: path.join(mobileDir, "sdk"),
    },
    nodeVersion: "22.13.0",
    put,
  };
}

function byId(result, id) {
  return result.checks.find((check) => check.id === id)?.ok;
}

test("missing wrapper, JDK, SDK and plugin block preflight without side effects", () => {
  const mobileDir = mkdtempSync(path.join(os.tmpdir(), "zoomouse-empty-preflight-"));
  temporary.push(mobileDir);
  const result = inspectAndroidReadiness({ mobileDir, env: {}, nodeVersion: "22.13.0" });
  assert.equal(result.readyForBuildAttempt, false);
  assert.equal(byId(result, "official-wrapper-jar"), false);
  assert.equal(byId(result, "sdk-platform-37"), false);
  assert.equal(byId(result, "jdk-release"), false);
  assert.equal(byId(result, "rn-gradle-plugin"), false);
});

test("all synthetic prerequisites except unverified wrapper JAR are detected", () => {
  const { mobileDir, env, nodeVersion } = fixture();
  const result = inspectAndroidReadiness({ mobileDir, env, nodeVersion });
  assert.equal(result.readyForBuildAttempt, false);
  assert.deepEqual(result.checks.filter(({ ok }) => !ok).map(({ id }) => id), ["official-wrapper-jar"]);
});

test("untrusted distribution, incorrect checksum and mismatched SDK roots fail closed", () => {
  const { mobileDir, env, nodeVersion, put } = fixture();
  put("android/gradle/wrapper/gradle-wrapper.properties",
    "distributionUrl=http\\://insecure.example/gradle.zip\ndistributionSha256Sum=invalid\n");
  const result = inspectAndroidReadiness({
    mobileDir,
    env: { ...env, ANDROID_SDK_ROOT: path.join(mobileDir, "other-sdk") },
    nodeVersion,
  });
  for (const id of ["wrapper-distribution", "wrapper-distribution-sha", "sdk-root", "sdk-platform-37", "ndk-28"]) {
    assert.equal(byId(result, id), false, `${id} must not be accepted`);
  }
  assert.equal(result.readyForBuildAttempt, false);
});

test("unsupported Node and JDK versions are rejected", () => {
  const { mobileDir, env, put } = fixture();
  put("jdk/release", 'JAVA_VERSION="1.8.0_412"\n');
  const result = inspectAndroidReadiness({ mobileDir, env, nodeVersion: "22.12.9" });
  assert.equal(byId(result, "node"), false);
  assert.equal(byId(result, "jdk-release"), false);
});
