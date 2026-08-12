# C++ — Machine Config Library

The C++ library lives in `cpp/include/machine_config/` and is **header-only**. Any C++17
project that links HighFive (HDF5 wrapper), nlohmann/json, and HDF5 ≥ 1.12 can include the
headers directly with no compilation step. All types live in the `machine_config` namespace.

← [Back to index](../USAGE.md)

---

## Contents

- [Build and install](#build-and-install)
- [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file)
- [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json)
- [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5)
- [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config)
- [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration)
- [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays)
- [Use case 10 — Validate a config against the JSON schema](#use-case-10--validate-a-config-against-the-json-schema)
- [CLI reference](#cli-reference)
- [Quickstart example](#quickstart-example)
- [Full workflow example](#full-workflow-example)
- [Running the C++ test suite](#running-the-c-test-suite)
- [Capability API (stable model facade)](#capability-api-stable-model-facade)

> Use cases 3, 5, and 7 (`config_from_dict`, `ConfigEditor`, `YamlConfigBuilder`) are
> Python-only conveniences with no C++ port.

---

## Capability API (stable model facade)

Preferred for applications. Include `machine_config/capabilities/file.hpp`. Full-model
get/set with `SetMode::Merge` (default) / `SetMode::Replace`:

```cpp
#include "machine_config/capabilities/file.hpp"

using machine_config::capabilities::openMachineConfig;
using machine_config::capabilities::SetMode;

auto opened = openMachineConfig("machine.h5");
auto file = std::static_pointer_cast<machine_config::capabilities::MachineConfigFileV10>(
    opened.value());
auto scanner = file->getScanner(0).value();
scanner.working_distance = 680.0;
file->setScanner(0, scanner);  // SetMode::Merge by default
std::string out = "out.h5";
file->save(&out);
file->close();
```

See [USAGE.md](../USAGE.md) and `schema/capabilities/`.

---

## Build and install

**Prerequisites**: CMake ≥ 3.20, a C++17 compiler (MSVC 19+, GCC 11+, Clang 14+), and HDF5
≥ 1.12 (vcpkg on Windows; build from source on Linux — see [cpp.yml](../.github/workflows/cpp.yml)).

```powershell
# PowerShell — Windows (vcpkg provides HDF5)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release `
  "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT\scripts\buildsystems\vcpkg.cmake"
cmake --build cpp/build --config Release
```

```bash
# Linux — after building HDF5 1.14.6 from source (see cpp.yml)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH=/path/to/hdf5-install \
  -DHDF5_ROOT=/path/to/hdf5-install
cmake --build cpp/build
```

After building, the CLI is at:
- Windows: `cpp/build/Release/machine_config_cli.exe`
- Linux:   `cpp/build/machine_config_cli`

All other dependencies (HighFive, nlohmann/json, CLI11, Catch2, json-schema-validator) are
fetched automatically via CMake `FetchContent`.

### As a dependency via FetchContent

The library is header-only — consumers only need CMake, a C++17 compiler, and HDF5 on their system:

```cmake
include(FetchContent)
FetchContent_Declare(machine_config
  GIT_REPOSITORY https://github.com/Trusted-Metal/Machine_Config_Library.git
  GIT_TAG        v0.2.0-rc.1
  SOURCE_SUBDIR  cpp)
FetchContent_MakeAvailable(machine_config)
target_link_libraries(your_target PRIVATE machine_config)
```

Track main (locks to HEAD on first configure; re-run cmake to advance):
```cmake
FetchContent_Declare(machine_config
  GIT_REPOSITORY https://github.com/Trusted-Metal/Machine_Config_Library.git
  GIT_TAG        main
  SOURCE_SUBDIR  cpp)
```

---

## Use case 1 — Parse a machine config file

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};
auto config = reader.parse();   // scalars + metadata only

std::cout << config.meta.machine_name << "\n";        // TM-LPBF-02: AconityMIDI+_OG
std::cout << config.meta.configuration_hash << "\n";  // 64-char hex
std::cout << config.optical_trains.size() << "\n";    // 2

for (size_t i = 0; i < config.optical_trains.size(); ++i) {
    const auto& s = config.optical_trains[i].scanner;
    std::cout << "Train " << (i + 1)
              << ": WD=" << s.working_distance.value_or(0.0)
              << "  offset=(" << s.scan_head_offset_x.value_or(0.0)
              << ", " << s.scan_head_offset_y.value_or(0.0) << ")\n";
}

// OPCUA — populated only when the file has an OPCUA group
if (config.opcua) {
    std::cout << config.opcua->client.server_url << "\n";
}
```

---

## Use case 2 — Export to canonical JSON

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};

// Scalar + metadata only (default), 2-space indent
std::string json = reader.toJson();
std::cout << json << "\n";

// Write to a file
std::ofstream{"output.json"} << json;

// Compact (no indentation)
std::string compact = reader.toJson(/*indent=*/0);
```

> **Binary data**: use `getCorrectionData()` / `getInverseCorrectionData()` /
> `getScanFieldCorrectionBytes()` for binary datasets. `toJson()` does not include them.

---

## Use case 4 — Write a config back to HDF5

```cpp
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};
auto config = reader.parse();

// All fields are plain value types — mutate directly
config.optical_trains[0].scanner.scan_head_offset_x = -91.5;
config.optical_trains[0].scanner.scan_head_offset_y =  24.0;

MachineConfigWriter{config}.write("output.h5");

// Verify the roundtrip
auto reread = MachineConfigReader{"output.h5"}.parse();
assert(config.meta.machine_name == reread.meta.machine_name);
assert(reread.optical_trains[0].scanner.scan_head_offset_x == -91.5);
```

> **Binary data in the writer**: if `correction_data` / `inverse_correction_data` are `nullopt`
> (config parsed with `parse()`, not `parseWithBinary()`), the writer writes zero-filled
> `(257, 257, 2)` datasets — identical behaviour to Python and Rust. Use `parseWithBinary()`
> before writing to preserve the original correction grids.

---

## Use case 6 — Generate a synthetic test config

`MockConfigBuilder` mirrors Python's and Rust's builders — same defaults, per-train geometry,
and Gaussian correction-grid formula (peak ≈ 2.0 at centre).

```cpp
#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"
using namespace machine_config;

// 2-laser config with ClearBox (default)
MockConfigBuilder{}.save("test_config.h5");

// Customise before saving
MockConfigBuilder b;
b.laser_count      = 1;
b.machine_name     = "TestMachine";
b.build_plate_x    = 400.0;
b.include_clearbox = false;
b.save("custom_config.h5");

// Build into memory without writing
auto config = MockConfigBuilder{}.build();
assert(config.optical_trains.size() == 2);
assert(config.meta.machine_name == "MockMachine");

// Verify the correction grid
auto cd = MachineConfigReader{"test_config.h5"}.getCorrectionData(0);
assert((cd.shape == std::array<size_t,3>{257, 257, 2}));
double peak = cd.data[(128 * 257 + 128) * 2 + 0];  // ≈ 2.0 (Gaussian peak)
assert(peak > 1.9 && peak < 2.1);
```

---

## Use case 8 — Read OPCUA telemetry configuration

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config_opcua.h5"};
auto config = reader.parse();

if (config.opcua) {
    const auto& opc = *config.opcua;
    std::cout << opc.client.server_url << "\n";
    std::cout << opc.client.auth_mode << "\n";
    for (const auto& [name, trigger] : opc.triggers) {
        std::cout << name << ": signal=" << trigger.signal.value_or("") << "\n";
    }
    if (opc.triggers_enabled)
        std::cout << "Triggers enabled: " << *opc.triggers_enabled << "\n";
}

// getRawGroup() — raw attribute map for any HDF5 path; {} if absent, never throws
auto client_attrs = reader.getRawGroup("OPCUA/Client");
std::cout << client_attrs["Server_URL"].get<std::string>() << "\n";

auto missing = reader.getRawGroup("does/not/exist");  // {}
assert(missing.empty());
```

---

## Use case 9 — Access ClearBox correction arrays

`CorrectionData` holds a flat row-major `std::vector<double>` and an
`std::array<size_t, 3> shape`. NaN values indicate out-of-field cells.

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};

// Forward grid, train 0 — flat buffer + shape, NaN preserved
CorrectionData cd = reader.getCorrectionData(0);
assert((cd.shape == std::array<size_t,3>{257, 257, 2}));

// Row-major: offset = (i * shape[1] + j) * shape[2] + k
size_t d1 = cd.shape[1], d2 = cd.shape[2];
double centre_x = cd.data[(128 * d1 + 128) * d2 + 0];
double centre_y = cd.data[(128 * d1 + 128) * d2 + 1];
if (!std::isnan(centre_x))
    std::cout << "Centre correction x=" << centre_x << "\n";

// Inverse grid
CorrectionData icd = reader.getInverseCorrectionData(0);

// Raw .fc3 bytes
auto bytes = reader.getScanFieldCorrectionBytes(0);  // std::vector<uint8_t>
std::cout << "fc3 size: " << bytes.size() << " bytes\n";  // 1138799 for train 0

// parseWithBinary() populates Grid3D (nested optional<double>) in the model
auto full = reader.parseWithBinary();
auto& cb = *full.optical_trains[0].optional_components.clearbox;
assert(cb.correction_data.has_value());
assert((*cb.correction_data).size() == 257);  // outer dimension
```

---

## Use case 10 — Validate a config against the JSON schema

`machine_config/schema.hpp` exposes a `validate()` free function backed by
[pboettch/json-schema-validator](https://github.com/pboettch/json-schema-validator) against
`schema/machine_config_v1.schema.json` (draft 7 keywords).

```cpp
#include "machine_config/reader.hpp"
#include "machine_config/schema.hpp"  // requires SCHEMA_DIR compile definition
using namespace machine_config;

// Validate reader output for a real fixture
auto j = nlohmann::json::parse(MachineConfigReader{"fixtures/reference_config.h5"}.toJson());
auto errors = validate(j);
if (errors.empty()) {
    std::cout << "Valid\n";
} else {
    for (const auto& e : errors)
        std::cerr << e << "\n";
}

// An empty object fails (missing meta/machine/optical_trains)
assert(!validate(nlohmann::json::object()).empty());
```

> `schema.hpp` requires `SCHEMA_DIR` to be defined as a compile-time string pointing to the
> directory containing `machine_config_v1.schema.json`. The CMake build sets this automatically
> for test executables; consumers must define it in their own build.

---

## CLI reference

```powershell
# PowerShell — Windows Release build
$cli = ".\cpp\build\Release\machine_config_cli.exe"

& $cli export-json fixtures/reference_config.h5
& $cli export-json fixtures/reference_config.h5 > output.json
& $cli write-hdf5 output.json reconstructed.h5
& $cli copy-hdf5 fixtures/reference_config.h5 copy.h5
& $cli correction-hash fixtures/reference_config.h5 --train 0
& $cli correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

```bash
# Linux
cli="cpp/build/machine_config_cli"
"$cli" export-json fixtures/reference_config.h5
"$cli" write-hdf5 output.json reconstructed.h5
"$cli" copy-hdf5 fixtures/reference_config.h5 copy.h5
"$cli" correction-hash fixtures/reference_config.h5 --train 0

# Windows (Git Bash)
cli="cpp/build/Debug/machine_config_cli.exe"
```

> `correction-hash` output is byte-identical to Python, Rust, and Node.js for the same
> file/train/direction. `copy-hdf5` is unique to C++ and preserves correction grids and
> `.fc3` bytes verbatim without deserialising through the model.

---

## Quickstart example

```powershell
# PowerShell — build first, then run
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release `
  "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT\scripts\buildsystems\vcpkg.cmake"
cmake --build cpp/build --config Release
.\cpp\build\Release\quickstart.exe
```

```bash
# Linux
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release
cmake --build cpp/build
cpp/build/quickstart

# Windows (Git Bash) — Debug build already present after tests
cpp/build/Debug/quickstart.exe
```

Expected output:

```
=== Machine Config Quickstart ===

Machine name   : ExampleDummy-2Train
Optical trains : 2
  Train 0  wd=670 mm  offset x=-87.5, y=23.5
           clearbox: present
  Train 1  wd=670 mm  offset x=87.5, y=-23.5
           clearbox: present
Correction grid: [257, 257, 2]   (train 0)

Written to     : mc_quickstart_tmp.h5

PASS
```

Source: [examples/quickstart/cpp/main.cpp](../examples/quickstart/cpp/main.cpp)

---

## Full workflow example

```powershell
# PowerShell
.\cpp\build\Release\full_workflow.exe
```

```bash
# Linux
cpp/build/full_workflow

# Windows (Git Bash)
cpp/build/Debug/full_workflow.exe
```

Expected output:

```
=== Full Workflow: Calibration Adjustment ===

Machine : ExampleDummy-2Train
Trains  : 2

Before calibration:
  Train 1  offset x=-87.5, y=23.5
           correction grid 257x257x2
  Train 2  offset x=87.5, y=-23.5
           correction grid 257x257x2

Written to : mc_full_workflow_tmp.h5

After calibration:
  Train 1  offset x=-91.5, y=24
  Train 2  offset x=91.5, y=-24

PASS
```

Source: [examples/full_workflow/cpp/main.cpp](../examples/full_workflow/cpp/main.cpp)

---

## Running the C++ test suite

```powershell
# PowerShell — from repo root
cmake --build cpp/build --config Debug
ctest --test-dir cpp/build -C Debug --output-on-failure
```

```bash
# Linux — from repo root
cmake --build cpp/build
ctest --test-dir cpp/build --output-on-failure

# Windows (Git Bash)
cmake --build cpp/build --config Debug
ctest --test-dir cpp/build -C Debug --output-on-failure
```

Expected: `100% tests passed, 0 tests failed out of 64`

| File | Tests | Coverage |
|---|---|---|
| `test_models.cpp` | 6 | JSON serialisation, nlohmann ADL round-trips |
| `test_reader.cpp` | 44 | Root attrs, machine, optical trains, scanner, ClearBox scalars, binary data, OPCUA (absent + all fields), synthetic fixture, `getRawGroup()` |
| `test_writer.cpp` | 7 | Scalar roundtrip, OPCUA roundtrip (field values + trigger content), OPCUA from-scratch construction, schema spot-check |
| `test_builder.cpp` | 6 | 1/2-laser roundtrip, plate dims, correction grid shape + value, no-clearbox |
| `test_schema.cpp` | 3 | Reference fixture validates, MockBuilder output validates, empty object fails |
