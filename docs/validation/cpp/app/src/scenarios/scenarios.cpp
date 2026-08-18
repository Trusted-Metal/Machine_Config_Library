// S-01-09/AV-01-08 scenario definitions for the C++ validation app
// (VALIDATION_PLAN.md §8). AV-09-11 live in cpp/tests/ instead — see
// docs/validation/cpp/results.md for why.
#include "scenarios.hpp"
#include "machine_config/machine_config.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <set>
#include <sstream>

// S-01: Read reference fixture, all scalar fields
//
// Expected: machine_name = "TM-LPBF-02: AconityMIDI+_OG"
//           build_plate_x ~= 250.0, build_plate_y ~= 250.0
//           len(optical_trains) = 2
//           train[0].scanner.working_distance ~= 670.0
//           train[0].scanner.scan_head_rotation ~= 0.0
//           train[1].scanner.scan_head_rotation ~= 180.0
//           configuration_hash: 64 hex characters
//           file_version: "1.0"
namespace s01 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    MachineConfig cfg;
    try {
        cfg = MachineConfigReader{path}.parse();
    } catch (const std::exception& e) {
        return {false, std::string("read failed: ") + e.what()};
    }

    if (cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG") {
        return {false, "machine_name: got '" + cfg.meta.machine_name + "'"};
    }
    if (!cfg.machine.build_plate_x || std::abs(*cfg.machine.build_plate_x - 250.0) > 0.001) {
        return {false, "build_plate_x mismatch"};
    }
    if (!cfg.machine.build_plate_y || std::abs(*cfg.machine.build_plate_y - 250.0) > 0.001) {
        return {false, "build_plate_y mismatch"};
    }
    if (cfg.optical_trains.size() != 2) {
        return {false, "optical_trains count: got " + std::to_string(cfg.optical_trains.size())};
    }

    const auto& wd = cfg.optical_trains[0].scanner.working_distance;
    if (!wd || std::abs(*wd - 670.0) > 0.1) {
        return {false, "train[0].working_distance mismatch"};
    }
    const auto& r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
    if (!r0 || std::abs(*r0) > 0.001) {
        return {false, "train[0].scan_head_rotation mismatch"};
    }
    const auto& r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
    if (!r1 || std::abs(*r1 - 180.0) > 0.001) {
        return {false, "train[1].scan_head_rotation mismatch"};
    }

    const auto& h = cfg.meta.configuration_hash;
    bool allHex = h.size() == 64 &&
        std::all_of(h.begin(), h.end(), [](unsigned char c) { return std::isxdigit(c); });
    if (!allHex) {
        return {false, "configuration_hash invalid: '" + h + "'"};
    }
    if (cfg.meta.file_version != "1.0") {
        return {false, "file_version: got '" + cfg.meta.file_version + "'"};
    }

    return {true, "all scalar fields match expected values"};
}

} // namespace s01

// S-02: Read reference fixture with binary data (correction grids)
namespace s02 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfigReader reader{path};
        CorrectionData cd = reader.getCorrectionData(0);
        CorrectionData icd = reader.getInverseCorrectionData(0);

        if (cd.shape != std::array<std::size_t, 3>{257, 257, 2}) {
            return {false, "correction_data shape mismatch"};
        }
        if (icd.shape != std::array<std::size_t, 3>{257, 257, 2}) {
            return {false, "inverse_correction_data shape mismatch"};
        }
        if (!std::any_of(cd.data.begin(), cd.data.end(), [](double v) { return std::isfinite(v); })) {
            return {false, "correction_data: no finite values"};
        }
        if (!std::any_of(icd.data.begin(), icd.data.end(), [](double v) { return std::isfinite(v); })) {
            return {false, "inverse_correction_data: no finite values"};
        }
        if (scenarios::bitwiseEqual(cd.data, icd.data)) {
            return {false, "correction_data and inverse_correction_data are identical"};
        }

        std::string hash = scenarios::sha256Hex(cd.data);
        return {true, "shapes OK, finite values OK, forward!=inverse, correction_data SHA-256=" + hash};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s02

// S-03: Read real AconityMIDI fixture
namespace s03 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path& realDir) {
    std::filesystem::path matched;
    for (const auto& entry : std::filesystem::directory_iterator(realDir)) {
        auto name = entry.path().filename().string();
        if (entry.path().extension() == ".h5" &&
            name.find("AconityMIDI") != std::string::npos &&
            name.find("OG_178") != std::string::npos) {
            matched = entry.path();
            break;
        }
    }
    if (matched.empty()) {
        return {false, "real AconityMIDI file not found in " + realDir.string()};
    }

    try {
        MachineConfig cfg = MachineConfigReader{matched}.parse();
        std::ostringstream oss;
        oss << "machine_name=\"" << cfg.meta.machine_name << "\""
            << " | file_version=\"" << cfg.meta.file_version << "\""
            << " | trains=" << cfg.optical_trains.size()
            << " | build_plate_x=" << (cfg.machine.build_plate_x ? std::to_string(*cfg.machine.build_plate_x) : "null")
            << " | build_plate_y=" << (cfg.machine.build_plate_y ? std::to_string(*cfg.machine.build_plate_y) : "null")
            << " | wd=" << (cfg.optical_trains[0].scanner.working_distance ? std::to_string(*cfg.optical_trains[0].scanner.working_distance) : "null")
            << " | rotation[0]=" << (cfg.optical_trains[0].scanner.scan_head_rotation ? std::to_string(*cfg.optical_trains[0].scanner.scan_head_rotation) : "null")
            << " | hash=" << cfg.meta.configuration_hash.substr(0, 16) << "...";
        return {true, oss.str()};
    } catch (const std::exception& e) {
        return {false, std::string("read failed: ") + e.what()};
    }
}

} // namespace s03

// S-04: Write modified config and verify field change survives roundtrip
namespace s04 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        auto origX = cfg.machine.build_plate_x;

        MachineConfig modified = cfg;
        modified.meta.machine_name = "VALIDATION_TEST_MACHINE";
        modified.machine.machine_name = "VALIDATION_TEST_MACHINE";

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{modified}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.meta.machine_name != "VALIDATION_TEST_MACHINE") {
            return {false, "machine_name not persisted: '" + rb.meta.machine_name + "'"};
        }
        if (rb.meta.file_version != "1.0") {
            return {false, "file_version changed: '" + rb.meta.file_version + "'"};
        }
        if (rb.machine.build_plate_x != origX) {
            return {false, "build_plate_x changed"};
        }

        return {true, "machine_name persisted, file_version and other fields unchanged"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s04

// S-05: Full binary roundtrip with correction hash verification
//
// parseWithBinary(), not parse(): this library splits scalars-only vs
// scalars+binary reads (like Rust, unlike Python's single always-binary
// parse()) — writing a config read via plain parse() would zero-fill the
// correction grid by design (see capabilities/v1_0/writer.hpp's
// "Absent Grid3D becomes a zero-filled default" comment).
namespace s05 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfigReader reader{path};
        MachineConfig cfg = reader.parseWithBinary();

        CorrectionData cdBefore = reader.getCorrectionData(0);
        CorrectionData icdBefore = reader.getInverseCorrectionData(0);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);

        MachineConfigReader reader2{tmp};
        CorrectionData cdAfter = reader2.getCorrectionData(0);
        CorrectionData icdAfter = reader2.getInverseCorrectionData(0);

        if (!scenarios::bitwiseEqual(cdBefore.data, cdAfter.data)) {
            return {false, "correction_data mismatch after roundtrip: " +
                scenarios::sha256Hex(cdBefore.data).substr(0, 16) + "... -> " +
                scenarios::sha256Hex(cdAfter.data).substr(0, 16) + "..."};
        }
        if (!scenarios::bitwiseEqual(icdBefore.data, icdAfter.data)) {
            return {false, "inverse_correction_data mismatch after roundtrip"};
        }

        return {true, "correction_data preserved: SHA-256=" + scenarios::sha256Hex(cdBefore.data)};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s05

// S-06: Build synthetic config with MockConfigBuilder and verify fields
namespace s06 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path&) {
    try {
        MockConfigBuilder builder;
        builder.laser_count = 2;
        MachineConfig cfg = builder.build();

        if (cfg.optical_trains.size() != 2) {
            return {false, "optical_trains count: " + std::to_string(cfg.optical_trains.size())};
        }

        const auto& r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
        const auto& r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
        if (!r0 || std::abs(*r0) > 0.001) return {false, "train[0].scan_head_rotation mismatch"};
        if (!r1 || std::abs(*r1 - 180.0) > 0.001) return {false, "train[1].scan_head_rotation mismatch"};
        if (cfg.meta.machine_name.empty()) return {false, "machine_name is empty"};

        const auto& clearbox = cfg.optical_trains[0].optional_components.clearbox;
        if (!clearbox) return {false, "clearbox is absent"};
        if (!clearbox->correction_data) return {false, "correction_data is absent"};

        GridCell center = (*clearbox->correction_data)[128][128][0];
        if (!center || !std::isfinite(*center) || std::abs(*center - 2.0) > 0.01) {
            return {false, "correction_data center: expected ~2.0"};
        }

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.optical_trains.size() != 2) {
            return {false, "readback trains: " + std::to_string(rb.optical_trains.size())};
        }
        if (rb.meta.machine_name != cfg.meta.machine_name) {
            return {false, "machine_name changed after roundtrip"};
        }

        return {true, "2-laser build OK, center~=2.0, roundtrip OK"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s06

// S-07: OPCUA config roundtrip
namespace s07 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config_opcua.h5";
    if (!std::filesystem::exists(path)) {
        return {false, "OPCUA fixture not found: " + path.string()};
    }

    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        if (!cfg.opcua) return {false, "opcua is absent after reading OPCUA fixture"};

        std::string origUrl = cfg.opcua->client.server_url;
        auto origTimeout = cfg.opcua->client.session_timeout;
        auto origTriggersEnabled = cfg.opcua->triggers_enabled;
        std::set<std::string> origNames;
        for (const auto& [name, trig] : cfg.opcua->triggers) origNames.insert(name);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();
        if (!rb.opcua) return {false, "opcua is absent after roundtrip"};

        if (rb.opcua->client.server_url != origUrl) {
            return {false, "server_url changed"};
        }
        if (rb.opcua->client.session_timeout != origTimeout) {
            return {false, "session_timeout changed"};
        }
        if (rb.opcua->triggers_enabled != origTriggersEnabled) {
            return {false, "triggers_enabled changed"};
        }

        std::set<std::string> rbNames;
        for (const auto& [name, trig] : rb.opcua->triggers) rbNames.insert(name);
        if (rbNames != origNames) {
            return {false, "trigger names changed"};
        }

        const std::string co = "Chamber Oxygen Level";
        auto oit = cfg.opcua->triggers.find(co);
        auto rit = rb.opcua->triggers.find(co);
        if (oit != cfg.opcua->triggers.end() && rit != rb.opcua->triggers.end()) {
            if (oit->second.signal != rit->second.signal || oit->second.subsystem != rit->second.subsystem) {
                return {false, "'" + co + "' signal/subsystem changed"};
            }
        }

        return {true, "OPCUA roundtrip OK: " + std::to_string(origNames.size()) + " triggers, url=\"" + origUrl + "\""};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s07

// S-08: Drastic field change to real file, verify adapter pipeline integrity
namespace s08 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path& realDir) {
    std::filesystem::path matched;
    for (const auto& entry : std::filesystem::directory_iterator(realDir)) {
        auto name = entry.path().filename().string();
        if (entry.path().extension() == ".h5" &&
            name.find("AconityMIDI") != std::string::npos &&
            name.find("OG_178") != std::string::npos) {
            matched = entry.path();
            break;
        }
    }
    if (matched.empty()) {
        return {false, "real AconityMIDI file not found in " + realDir.string()};
    }

    try {
        MachineConfig cfg = MachineConfigReader{matched}.parse();

        MachineConfig modified = cfg;
        modified.meta.machine_name = "MODIFIED_ACONITY_VALIDATION";
        modified.machine.machine_name = "MODIFIED_ACONITY_VALIDATION";
        modified.machine.build_plate_x = 350.0;

        OpticalTrain newTrain = cfg.optical_trains[1];
        newTrain.train_id = "Optical_Train_03";
        newTrain.scanner.scan_head_rotation = 90.0;
        newTrain.optional_components.clearbox = std::nullopt;
        modified.optical_trains.push_back(newTrain);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{modified}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.optical_trains.size() != 3) {
            return {false, "optical_trains: expected 3, got " + std::to_string(rb.optical_trains.size())};
        }
        if (!rb.machine.build_plate_x || std::abs(*rb.machine.build_plate_x - 350.0) > 0.001) {
            return {false, "build_plate_x mismatch"};
        }
        const auto& r2 = rb.optical_trains[2].scanner.scan_head_rotation;
        if (!r2 || std::abs(*r2 - 90.0) > 0.001) {
            return {false, "train[2].scan_head_rotation mismatch"};
        }
        if (rb.optical_trains[2].optional_components.clearbox.has_value()) {
            return {false, "train[2].clearbox should be absent"};
        }
        if (rb.meta.machine_name != "MODIFIED_ACONITY_VALIDATION") {
            return {false, "machine_name: got '" + rb.meta.machine_name + "'"};
        }
        if (rb.meta.file_version != "1.0") {
            return {false, "file_version changed: '" + rb.meta.file_version + "'"};
        }

        return {true, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s08

// S-09: Public type export surface
//
// Every public type must resolve from the single umbrella header
// (no internal include paths) and be usable, not just nameable.
// See VALIDATION_PLAN.md §8 S-09 and §9.5.
namespace s09 {

using namespace machine_config;

// OpcuaConfig is never populated by MockConfigBuilder's default build (no
// OPCUA fixture involved here) — type-annotating a never-called parameter is
// exactly what VALIDATION_PLAN.md §8 S-09 allows: "Construct OR
// type-annotate a variable with each type."
static void typeCheckOpcua(const OpcuaConfig&) {}

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path&) {
    (void)&typeCheckOpcua;

    MockConfigBuilder builder;
    builder.laser_count = 2;
    MachineConfig config = builder.build();

    const MachineConfigMeta& meta = config.meta;
    const Machine& machine = config.machine;

    if (config.optical_trains.empty()) {
        return {false, "builder produced zero optical trains"};
    }
    const OpticalTrain& train = config.optical_trains[0];
    const Scanner& scanner = train.scanner;
    const LightSource& lightSource = train.light_source;
    const Collimator& collimator = train.collimator;
    const ScannerCard& scannerCard = train.scanner_card;
    const OptionalComponents& optionalComponents = train.optional_components;

    if (!optionalComponents.clearbox.has_value()) {
        return {false, "expected builder to include a clearbox by default"};
    }
    const ClearBox& clearbox = *optionalComponents.clearbox;

    if (!train.scan_field_correction_file.has_value()) {
        return {false, "expected builder to include an SFCF by default"};
    }
    const ScanFieldCorrectionFile& sfcf = *train.scan_field_correction_file;

    MachineConfigReader reader{std::filesystem::path{"unused"}};
    MachineConfigWriter writer{config};
    (void)reader;
    (void)writer;

    if (meta.machine_name.empty() || machine.manufacturer.empty() ||
        scanner.manufacturer.empty() || lightSource.manufacturer.empty() ||
        collimator.manufacturer.empty() || scannerCard.manufacturer.empty() ||
        clearbox.ip_address.empty() || sfcf.document_name.empty()) {
        return {false, "one or more constructed fields were unexpectedly empty"};
    }

    return {true, "all public types resolve and are usable from the umbrella header"};
}

} // namespace s09

// AV-01: Reader rejects unknown File_Version
//
// No typed exception hierarchy exists for the plain reader/writer (see
// VALIDATION_PLAN.md §9.5 "No typed exception hierarchy") — every error path
// throws a bare std::runtime_error. This inspects .what() for the version
// string rather than a typed error field.
namespace av01 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "v2_0_unknown.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for File_Version='2.0'"};
    } catch (const std::exception& e) {
        std::string msg = e.what();
        if (msg.find("2.0") != std::string::npos) {
            return {true, std::string("error raised naming version '2.0': ") + msg};
        }
        return {false, std::string("error raised but did not name version '2.0': ") + msg};
    }
}

} // namespace av01

// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — this records the actual behavior rather than
// asserting one, mirroring every other language's AV-02.
namespace av02 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "missing_version.h5");
    try {
        MachineConfig cfg = MachineConfigReader{fixture}.parse();
        return {true, "missing File_Version dispatches OK, file_version='" + cfg.meta.file_version + "'"};
    } catch (const std::exception& e) {
        return {true, std::string("missing File_Version raises: ") + e.what()};
    }
}

} // namespace av02

// AV-03: v1.0 reader encountering a v1.1 file fails loudly
namespace av03 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "v1_1_simulated.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for File_Version='1.1'"};
    } catch (const std::exception& e) {
        std::string msg = e.what();
        if (msg.find("1.1") != std::string::npos) {
            return {true, std::string("error raised naming version '1.1': ") + msg};
        }
        return {false, std::string("error raised but did not name version '1.1': ") + msg};
    }
}

} // namespace av03

// AV-04: Reader returns an error when required group (Machine/) is absent
namespace av04 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "missing_machine_group.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for missing Machine/ group"};
    } catch (const std::exception& e) {
        return {true, std::string("error raised for missing Machine/ group: ") + e.what()};
    }
}

} // namespace av04

// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully
namespace av05 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "corrupt_scalar.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for corrupt Build_Plate_X_Dimension"};
    } catch (const std::exception& e) {
        return {true, std::string("error raised for corrupt scalar: ") + e.what()};
    }
}

} // namespace av05

// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")
namespace av06 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "version_whitespace.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"};
    } catch (const std::exception& e) {
        return {false, e.what()};
    }
}

} // namespace av06

// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring every other
// language's AV-07.
namespace av07 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "empty_version.h5");
    try {
        MachineConfig cfg = MachineConfigReader{fixture}.parse();
        return {true, "empty File_Version dispatches OK, file_version='" + cfg.meta.file_version + "'"};
    } catch (const std::exception& e) {
        return {true, std::string("empty File_Version raises: ") + e.what()};
    }
}

} // namespace av07

// AV-08: File_Version string survives write -> read unchanged
namespace av08 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        std::string origVersion = cfg.meta.file_version;

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();
        std::string rbVersion = rb.meta.file_version;

        if (rbVersion != "1.0") {
            return {false, "file_version after roundtrip: expected '1.0', got '" + rbVersion + "'"};
        }
        if (rbVersion != origVersion) {
            return {false, "file_version changed: '" + origVersion + "' -> '" + rbVersion + "'"};
        }

        return {true, "File_Version survives roundtrip unchanged: '" + rbVersion + "'"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace av08
