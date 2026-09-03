# C++ Packaging — Results and Deviation from the Original §10 Plan

**Date:** 2026-08-18
**Compiler:** MSVC 19.43.34809.0 (Visual Studio 17 2022)
**Platform:** Windows 11

---

## Deviation from the original plan (read this first)

§10 as originally written described a **fully self-contained static tarball** — headers, a
static `.a`/`.lib`, and a bundled copy of HDF5 itself, so a consumer needs nothing else
installed. That is not what was built here. After discussion, the decision was made to build
the **"thin"** package instead: `machine_config`'s own headers are installed, plus a generated
`MachineConfigConfig.cmake` — but HighFive's, nlohmann_json's, and HDF5's files are
**not** bundled. The generated config instead does:

```cmake
find_dependency(HighFive)
find_dependency(nlohmann_json)
```

**Why the change:** the stated reason for wanting a C++ package at all is the eventual
possibility of a vcpkg port. vcpkg ports don't bundle their dependencies — they *declare*
them (`highfive`, `nlohmann-json`, and `hdf5` all already exist as real vcpkg ports), vcpkg
builds each into one shared per-triplet install tree, and its toolchain file makes
`find_package()` resolve all of them automatically for any consumer. A package that bundled
private copies of HighFive's/nlohmann_json's headers would actively fight that model —
duplicate copies of the same library sitting next to the "real" vcpkg-managed ones. The thin
design is not a scoped-down placeholder; it's a rehearsal of the exact shape a real vcpkg
port's exported `Config.cmake` needs to have. §2's and §10's original "self-contained,
without installing HDF5 separately" wording predates this direction and should be read as
superseded by this document, not as the still-current goal.

One consequence worth being explicit about: **this package is not yet what most people mean
by "a static tarball you can hand someone with nothing else installed."** It still requires a
consumer to have HighFive, nlohmann_json, and HDF5 separately discoverable — today that means
manually building/installing each, or (once it exists) a vcpkg environment. That gap is
intentional and matches where this is headed, not an oversight.

---

## What was actually needed (vs. the explanation given before starting)

The explanation given to the user before starting held up almost exactly as described:
1. `install()` rules telling CMake what to copy (`machine_config`'s headers) — needed, as expected.
2. Registering `machine_config` as an export set — needed, as expected, and turned out to
   need zero extra work for the *linked* dependencies (HighFive, nlohmann_json) — see below.
3. A generated `MachineConfigConfig.cmake` with `find_dependency` calls — needed, as expected.
4. A generated `MachineConfigConfigVersion.cmake` via `write_basic_package_version_file` — needed, as expected.
5. A real, fresh, no-repo-context consumer test — needed, and it's what caught the one real bug (below).

---

## Two real problems hit, and how they were found

### 1. A real CMake export error — not hypothetical

Before writing any code, there was a genuine open question: since `machine_config` links
against `HighFive` and `nlohmann_json::nlohmann_json` — both built in the same CMake run via
`FetchContent`, not found via their own `find_package()` calls — would CMake's
`install(EXPORT ...)` machinery even allow exporting `machine_config` at all, or would it
throw the well-known "target ... requires target ... that is not in any export set" error?

This was checked directly rather than guessed: both HighFive's and nlohmann_json's own
`CMakeLists.txt` were read to confirm each already does its own proper `install(EXPORT ...)` +
generates its own `Config.cmake` (nlohmann_json's gated behind a `JSON_Install` option that
defaults **off** when it's a sub-project — fixed by forcing it `ON` in `cpp/CMakeLists.txt`
before `FetchContent_MakeAvailable(nlohmann_json)`). Because both dependencies are already
properly export-capable, `install(EXPORT MachineConfigTargets)` succeeded on the very first
real attempt with no export-set error at all. The worry was legitimate; the outcome, once
checked against the real source instead of assumed, was that no fix was needed for this part.

### 2. A real, second CMake error — this one did require a fix

The first real `cmake -S cpp -B cpp/pkg-build` attempt failed with:

```
CMake Error in CMakeLists.txt:
  Target "machine_config" INTERFACE_INCLUDE_DIRECTORIES property contains
  path:
    "C:/Users/ChrisParham/Desktop/Repo/Machine_Config_Library/cpp/include"
  which is prefixed in the source directory.
```

Root cause: `target_include_directories(machine_config INTERFACE include)` used a bare
relative path. For this repo's own normal build, CMake quietly resolves that relative to
`cpp/`, so it always worked. `install(EXPORT ...)` refuses to export a target whose include
path is a raw absolute path into the *source* tree, since that path is meaningless for a real
installed consumer. Fixed with the standard two-value pattern (the same one HighFive's own
`CMakeLists.txt` already uses for the identical problem):

```cmake
target_include_directories(machine_config INTERFACE
  "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"
  "$<INSTALL_INTERFACE:include>")
```

After this fix, `cmake -S cpp -B cpp/pkg-build -DCMAKE_INSTALL_PREFIX=cpp/pkg-install`
configured cleanly, and `cmake --install cpp/pkg-build` installed:
- `cpp/pkg-install/include/machine_config/**` (this library's own headers)
- `cpp/pkg-install/include/nlohmann/**`, `cpp/pkg-install/include/highfive/**` (installed as a
  side effect of building `machine_config` in this same configure run — not something the
  thin design deliberately bundles; a real vcpkg environment would put these in the shared
  triplet tree instead of alongside `machine_config`)
- `cpp/pkg-install/lib/cmake/MachineConfig/{MachineConfigTargets.cmake, MachineConfigConfig.cmake, MachineConfigConfigVersion.cmake}`

(`cmake --build cpp/pkg-build --config Release --target machine_config` printed an MSBuild
error, `MSB1009: Project file does not exist` — harmless, not a real problem: `machine_config`
is `INTERFACE`-only, so Visual Studio's generator has no project file for it at all. There was
nothing to compile; the install step below it succeeded regardless.)

---

## Fresh-consumer verification (the real proof)

A genuinely separate project — no `add_subdirectory` into the repo, no reference to the repo's
source tree anywhere — was created at
`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\CppTarballConsumer\`
(distinct from the existing `Cpp\` folder there, which is the *from-source* app used for
S-01–09/AV-01–08 and does use `add_subdirectory`). Its entire `CMakeLists.txt`:

```cmake
find_package(MachineConfig REQUIRED)
add_executable(consumer_app src/main.cpp)
target_link_libraries(consumer_app PRIVATE MachineConfig::machine_config)
```

```
cmake -S . -B build -DCMAKE_PREFIX_PATH="C:/Users/ChrisParham/Desktop/Repo/Machine_Config_Library/cpp/pkg-install"
```

**Verbatim configure output:**
```
-- HIGHFIVE 2.10.0: (Re)Detecting Highfive dependencies (HIGHFIVE_USE_INSTALL_DEPS=NO)
-- Found HDF5: hdf5::hdf5-shared (found version "1.14.6")
-- Found nlohmann_json: C:/Users/ChrisParham/Desktop/Repo/Machine_Config_Library/cpp/pkg-install/share/cmake/nlohmann_json/nlohmann_jsonConfig.cmake (found version "3.11.3")
-- Configuring done (2.3s)
-- Generating done (0.0s)
```

Build succeeded on the first attempt after the include-directory fix above (no further errors).
Running the built consumer against the real reference fixture:

```
[PASS] machine_name="TM-LPBF-02: AconityMIDI+_OG" file_version="1.0" trains=2
```

This is the actual proof: a project that has never seen this repository's source, given only
the installed package and a `CMAKE_PREFIX_PATH`, successfully resolves `find_package(MachineConfig)`,
links against it, and correctly parses a real HDF5 file.

**Note on this specific run vs. a real vcpkg environment:** `HDF5` resolved here because the
HDF Group's own Windows installer (already present on this machine) ships a proper CMake
config package alongside its static and shared libs — no vcpkg needed for *this* local proof.
CI (and a real vcpkg port, eventually) will need its own equivalent resolution path (vcpkg
manifest mode, or a from-source HDF5 build on Linux) — that machinery is not part of this
change and is called out separately below.

---

## CI verification step — added

`.github/workflows/cpp.yml`'s existing `test` job (fires on every push/PR, matrix over
Ubuntu/Windows) now installs `machine_config` from that job's own already-configured
`cpp/build` — no second HDF5/vcpkg resolution path was needed; the install step just packages
what that build already resolved — then configures, builds, and runs the ported fresh-consumer
test (`docs/validation/cpp/tarball/consumer/`, ported into the repo from the external proof
above) against `fixtures/reference_config.h5`, failing the job if its output doesn't start with
`[PASS]`.

**This is a verification-only step.** It installs into `${{ github.workspace }}/cpp/pkg-install`,
which is thrown away when the job ends — nothing is uploaded or persisted. It catches
regressions to the packaging on every PR; it does not produce a distributable artifact.

**Verified locally before landing**, using the exact commands the YAML runs (not just read and
assumed correct): `cmake --install cpp/build --config Release --prefix <path>` against the
*main* dev build directory specifically (the local proof earlier in this document used a
separate `cpp/pkg-build` — re-tested against `cpp/build` since that's what CI actually has),
then the ported consumer configured against that install output:

```
-- HIGHFIVE 2.10.0: (Re)Detecting Highfive dependencies (HIGHFIVE_USE_INSTALL_DEPS=NO)
-- Found HDF5: hdf5::hdf5-shared (found version "1.14.6")
-- Found nlohmann_json: .../cpp/ci-sim-install/share/cmake/nlohmann_json/nlohmann_jsonConfig.cmake (found version "3.11.3")
```
```
[PASS] machine_name="TM-LPBF-02: AconityMIDI+_OG" file_version="1.0" trains=2
```

The Windows-specific half of the CI step (vcpkg toolchain file resolution) could not be
exercised locally the same way — this machine resolves HDF5 via the HDF Group's own installer,
not vcpkg — so that half is verified by inspection/consistency with the job's existing,
already-working vcpkg setup (used by the main build a few steps earlier in the same job) rather
than by an independent local run. Worth watching on the first real CI run.

---

## Version fidelity — a second real CMake bug, found before it ever shipped

Before the CD step could be built, a real correctness problem was found and fixed:
`sync-version.mjs` was about to write the repo's actual current version (`0.2.0-rc.4` — a
semver prerelease) directly into `cpp/CMakeLists.txt`'s `project(... VERSION ...)` field.
Verified directly, not assumed: `project(x VERSION 0.2.0-rc.4)` hard-errors —
`VERSION "0.2.0-rc.4" format invalid.` — because CMake's `VERSION` field only accepts numeric
dotted components, no semver prerelease/build suffix at all. Since every commit to `main`
releases as an `rc.N` prerelease (`.releaserc.json`'s branch config), this would have broken
*every* future `cmake` configure the first time a real release ran.

Fixed by truncating to the numeric-only portion for `project()`'s field
(`version.split(/[-+]/)[0]`, verified against combined prerelease+build-metadata strings too),
while preserving the *full* version separately: a new `cpp/cmake/Version.cmake` (committed,
kept in sync by `sync-version.mjs`) feeds a `configure_file()`-generated
`include/machine_config/version.hpp` exposing `machine_config::kVersionString` — the one place
the complete version string (e.g. `"0.2.0-rc.4"`) is available to C++ code, since CMake's own
`PROJECT_VERSION` structurally cannot hold it. Also added a `--version` flag to
`machine_config_cli` for the same reason. Confirmed, from a further follow-up question, that
this loss of precision in CMake's *native* version field is inherent to the tool — read
CMake's own `BasicConfigVersion-AnyNewerVersion.cmake.in` directly: its `VERSION_LESS`/
`VERSION_GREATER` comparisons are purely numeric, with no concept of prerelease ordering at
all, so no cleverer encoding would have preserved that specific comparison's fidelity anyway.

**Verified end-to-end**, not just unit-by-unit: ran `sync-version.mjs` for real, reconfigured,
rebuilt, ran `machine_config_cli --version` (`0.2.0-rc.4`), packaged, extracted the resulting
tarball, and confirmed the *installed* `version.hpp` — reached by a fresh consumer with zero
repo context — reports the same value: `package_version="0.2.0-rc.4"`.

---

## CD artifact-producing step — added

`.releaserc.json`'s `publishCmd` now also runs `scripts/package-cpp.sh` (after Node's `npm pack`
and Python's `python -m build`), which reconfigures `cpp/build` (deliberately *after*
`prepareCmd`'s `sync-version` has already bumped the version — a pre-bump configure would have
baked the *previous* release's version into the package, a silent correctness bug, not a
cosmetic one), installs, and packages `cpp/machine-config-cpp-<version>.tar.gz`.
`@semantic-release/github`'s `assets` list now includes that glob.

**No failure-guarding, by deliberate choice:** a broken C++ package blocks the release the same
way a broken Node/Python package already does today — confirmed the existing `publishCmd`
(`(cd nodejs && npm pack) && python -m build python/`) has never had any guard either, so this
isn't introducing an inconsistency. This was a real design fork, checked against semantic-
release's own source before deciding: `lib/definitions/plugins.js` shows every lifecycle step
*except* `publish` sets `settleAll: true`; `publish` uses `pipeline.js`'s sequential default,
which throws (and skips every later plugin in that step — `git`, `github`, both listed after
`exec`) the instant one plugin's publish hook fails. `@semantic-release/github`'s own
`publish.js` was also checked and *does* degrade gracefully for a merely-missing asset file —
but that safety net is irrelevant if `exec` throws first and `github` never runs at all. Given
the choice between failing loudly (current decision) and swallowing the failure, loud was
chosen deliberately, not by default.

**`release.yml` prerequisite added:** the `release` job had zero C++/CMake/HDF5 setup at all —
`cmake -S cpp -B cpp/build` (the first command in `package-cpp.sh`) transitively calls
`find_package(HDF5 REQUIRED)` via HighFive, which would otherwise fail unconditionally on every
run, not just when something is genuinely broken. Added the same HDF5-1.14.6-from-source build
+ cache `cpp.yml`'s Ubuntu leg already uses (this job only runs on `ubuntu-latest`, so the
artifact is Linux-built — acceptable since the thin package has no compiled code of its own).

**Verified as fully as possible without a real release run:** `scripts/package-cpp.sh` was run
locally end-to-end (`HDF5_INSTALL` pointed at this machine's real HDF5 install, standing in for
the CI-built one), producing a real tarball; extracted it and confirmed `version.hpp` inside
carries the correct version. `.releaserc.json` and `release.yml` were both validated as
syntactically correct (JSON parse; YAML parse via `js-yaml`, confirming the resulting step
order). What could *not* be verified locally: an actual `npx semantic-release` run (needs a
real git tag/GitHub token/release context) and the Ubuntu-CI-specific HDF5-from-source build
path (this machine has HDF5 from the HDF Group's own installer, not built from source the way
CI will). Both are real, disclosed gaps — the first real signal will be the next push to `main`.

---

## What's still open (not done in this pass)

- **A real vcpkg port** (`portfile.cmake` + `vcpkg.json`) is the natural next step this design
  was built toward, but is out of scope for this pass — explicitly deferred, per the original
  conversation.
- **The Windows-specific half of `cpp.yml`'s verification step** (vcpkg toolchain resolution)
  still hasn't been exercised by an independent local run — see the CI verification section
  above.
- **An actual end-to-end `npx semantic-release` dry run** hasn't happened — the CD step is
  built and locally verified piece-by-piece, but its first real, full-pipeline exercise will be
  the next push to `main` (which, per `.releaserc.json`'s branch config, releases as an `rc.N`
  prerelease — not a distant, hypothetical `release`-branch event).
