#!/usr/bin/env node
/**
 * Sync version from root package.json to all language manifests.
 * Called by semantic-release via `npm run sync-version`.
 *
 * Source of truth: package.json (updated by @semantic-release/npm before this runs)
 * Targets:
 *   rust/Cargo.toml       — version = "x.y.z"
 *   python/pyproject.toml — version = "x.y.z"
 *   nodejs/package.json   — "version": "x.y.z"
 *
 * To add a new language, add a block below following the same pattern.
 */

import { readFileSync, writeFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, "..");

function readFile(path) {
  return readFileSync(path, "utf8");
}

function writeFile(path, content) {
  writeFileSync(path, content, "utf8");
}

function main() {
  // Read version from root package.json (already updated by @semantic-release/npm)
  const packageJson = JSON.parse(readFile(join(rootDir, "package.json")));
  const version = packageJson.version;

  if (!version) {
    console.error("❌ No version found in package.json");
    process.exit(1);
  }

  console.log(`📦 Syncing version: ${version}`);

  // --- rust/Cargo.toml ---
  const cargoPath = join(rootDir, "rust", "Cargo.toml");
  const cargo = readFile(cargoPath);
  const oldCargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "?";
  writeFile(cargoPath, cargo.replace(/^(version\s*=\s*)"[^"]+"/m, `$1"${version}"`));
  console.log(`  ✓ rust/Cargo.toml: ${oldCargoVersion} → ${version}`);

  // --- python/pyproject.toml ---
  const pyprojectPath = join(rootDir, "python", "pyproject.toml");
  const pyproject = readFile(pyprojectPath);
  const oldPyVersion = pyproject.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "?";
  writeFile(pyprojectPath, pyproject.replace(/^(version\s*=\s*)"[^"]+"/m, `$1"${version}"`));
  console.log(`  ✓ python/pyproject.toml: ${oldPyVersion} → ${version}`);

  // --- nodejs/package.json ---
  const nodePkgPath = join(rootDir, "nodejs", "package.json");
  const nodePkg = JSON.parse(readFile(nodePkgPath));
  const oldNodeVersion = nodePkg.version ?? "?";
  nodePkg.version = version;
  writeFile(nodePkgPath, JSON.stringify(nodePkg, null, 2) + "\n");
  console.log(`  ✓ nodejs/package.json: ${oldNodeVersion} → ${version}`);

  // --- cpp/CMakeLists.txt (project() VERSION — numeric-only, truncated) ---
  // CMake's project(... VERSION ...) field is NOT semver — it only accepts
  // numeric dotted components (major[.minor[.patch[.tweak]]]), verified
  // directly: `project(x VERSION 0.2.0-rc.4)` hard-errors with
  // 'VERSION "0.2.0-rc.4" format invalid.' A prerelease/build suffix would
  // break every future `cmake` configure the moment this ran, so it's
  // stripped here. The FULL version (including any suffix) still goes to
  // cpp/cmake/Version.cmake below — nothing is lost, it just can't live in
  // this specific CMake field.
  const cmakePath = join(rootDir, "cpp", "CMakeLists.txt");
  const cmake = readFile(cmakePath);
  const cmakeVersion = version.split(/[-+]/)[0];
  const oldCmakeVersion = cmake.match(/^project\([^)]+VERSION\s+([^\s)]+)/m)?.[1] ?? "?";
  writeFile(cmakePath, cmake.replace(/^(project\([^)]+VERSION\s+)[^\s)]+/m, `$1${cmakeVersion}`));
  console.log(`  ✓ cpp/CMakeLists.txt (project VERSION): ${oldCmakeVersion} → ${cmakeVersion}`);

  // --- cpp/cmake/Version.cmake (full, untruncated version) ---
  // Feeds include/machine_config/version.hpp.in via configure_file() — see
  // that file for why this exists separately from PROJECT_VERSION.
  const cppVersionCmakePath = join(rootDir, "cpp", "cmake", "Version.cmake");
  const cppVersionCmake = readFile(cppVersionCmakePath);
  const oldFullVersion = cppVersionCmake.match(/MACHINE_CONFIG_FULL_VERSION\s+"([^"]+)"/)?.[1] ?? "?";
  writeFile(
    cppVersionCmakePath,
    cppVersionCmake.replace(/(MACHINE_CONFIG_FULL_VERSION\s+)"[^"]+"/, `$1"${version}"`)
  );
  console.log(`  ✓ cpp/cmake/Version.cmake (full version): ${oldFullVersion} → ${version}`);

  console.log(`✅ All versions synced to ${version}`);
}

main();
