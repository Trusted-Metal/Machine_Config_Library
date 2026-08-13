// Catch2 writer tests — §4.14 acceptance criteria.
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>
#include <nlohmann/json.hpp>
#include <picosha2.h>

#include <filesystem>
#include <stdexcept>
#include <string>

#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

static const std::string REF      = std::string(FIXTURES_DIR) + "/reference_config.h5";
static const std::string OPCUA_REF= std::string(FIXTURES_DIR) + "/reference_config_opcua.h5";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static std::filesystem::path tmpPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_writer_test_" + tag + ".h5");
}

// Build a minimal valid MachineConfig with no ClearBox and no OPCUA.
static MachineConfig makeMinimalConfig() {
    MachineConfig cfg;
    cfg.meta.schema_version     = "v1";
    cfg.meta.machine_name       = "MinimalMachine";
    cfg.meta.manufacturer       = "Acme";
    cfg.meta.model              = "M1";
    cfg.meta.serial_number      = "SN-0001";
    cfg.meta.file_version       = "1.0";
    cfg.meta.export_date        = "2024-01-01T00:00:00Z";
    cfg.meta.configuration_hash = std::string(64, '0');
    cfg.meta.extra              = nlohmann::json::object();

    cfg.machine.machine_name  = "MinimalMachine";
    cfg.machine.manufacturer  = "Acme";
    cfg.machine.model         = "M1";
    cfg.machine.serial_number = "SN-0001";
    cfg.machine.build_plate_x = 200.0;
    cfg.machine.build_plate_x_unit = "mm";
    cfg.machine.build_plate_y = 200.0;
    cfg.machine.build_plate_y_unit = "mm";

    OpticalTrain t;
    t.train_id = "Optical_Train_01";
    t.scanner.manufacturer  = "ScanMfr"; t.scanner.model = "SM1"; t.scanner.serial_number = "S-1";
    t.scanner.axis_configuration = "2D";
    t.light_source.manufacturer = "LSMfr";  t.light_source.model = "LS1"; t.light_source.serial_number = "L-1";
    t.collimator.manufacturer   = "ColMfr"; t.collimator.model = "C1";   t.collimator.serial_number = "C-1";
    t.scanner_card.manufacturer = "SCMfr";  t.scanner_card.model = "K1"; t.scanner_card.serial_number = "K-1";
    cfg.optical_trains.push_back(t);
    return cfg;
}

// ---------------------------------------------------------------------------
// RoundtripAllScalarFields
// Read reference fixture → write to temp → read back → compare key fields.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripAllScalarFields") {
    auto out = tmpPath("roundtrip");
    MachineConfigReader src{REF};
    auto orig = src.parse();

    REQUIRE_NOTHROW(MachineConfigWriter{orig}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    // meta
    REQUIRE(rb.meta.machine_name       == orig.meta.machine_name);
    REQUIRE(rb.meta.configuration_hash == orig.meta.configuration_hash);
    REQUIRE(rb.meta.file_version       == orig.meta.file_version);
    REQUIRE(rb.meta.manufacturer       == orig.meta.manufacturer);

    // machine
    REQUIRE(rb.machine.manufacturer == orig.machine.manufacturer);
    REQUIRE(rb.machine.model        == orig.machine.model);
    REQUIRE(rb.machine.build_plate_x.has_value());
    REQUIRE_THAT(*rb.machine.build_plate_x, Catch::Matchers::WithinRel(*orig.machine.build_plate_x));
    REQUIRE(rb.machine.build_plate_x_unit == orig.machine.build_plate_x_unit);

    // trains
    REQUIRE(rb.optical_trains.size() == orig.optical_trains.size());

    const auto& t0  = orig.optical_trains[0];
    const auto& rb0 = rb.optical_trains[0];
    REQUIRE(rb0.train_id == t0.train_id);
    REQUIRE(rb0.thermal_lensing_passed == t0.thermal_lensing_passed);
    REQUIRE_THAT(*rb0.scanner.scan_head_offset_x,
                 Catch::Matchers::WithinRel(*t0.scanner.scan_head_offset_x));
    REQUIRE_THAT(*rb0.scanner.working_distance,
                 Catch::Matchers::WithinRel(*t0.scanner.working_distance));

    // ClearBox scalar
    REQUIRE(rb0.optional_components.clearbox.has_value());
    REQUIRE(rb0.optional_components.clearbox->ip_address ==
            t0.optional_components.clearbox->ip_address);
    REQUIRE(rb0.optional_components.clearbox->data_port ==
            t0.optional_components.clearbox->data_port);

    // SFCF
    REQUIRE(rb0.scan_field_correction_file.has_value());
    REQUIRE(rb0.scan_field_correction_file->document_name ==
            t0.scan_field_correction_file->document_name);
    REQUIRE(rb0.scan_field_correction_file->file_size ==
            t0.scan_field_correction_file->file_size);

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// RoundtripWithoutClearBox
// Minimal config with no ClearBox survives a write → read cycle.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripWithoutClearBox") {
    auto out = tmpPath("no_clearbox");
    auto cfg = makeMinimalConfig();
    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    REQUIRE(rb.meta.machine_name == "MinimalMachine");
    REQUIRE(rb.optical_trains.size() == 1);
    REQUIRE_FALSE(rb.optical_trains[0].optional_components.clearbox.has_value());
    REQUIRE_FALSE(rb.opcua.has_value());

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// RoundtripWithOpcua
// Read OPCUA fixture → write → read back → verify OPCUA fields preserved.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripWithOpcua") {
    auto out = tmpPath("opcua");
    MachineConfigReader src{OPCUA_REF};
    auto orig = src.parse();
    REQUIRE(orig.opcua.has_value());

    REQUIRE_NOTHROW(MachineConfigWriter{orig}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    REQUIRE(rb.opcua.has_value());
    REQUIRE(rb.opcua->client.server_url    == orig.opcua->client.server_url);
    REQUIRE(rb.opcua->client.session_timeout == orig.opcua->client.session_timeout);
    REQUIRE(rb.opcua->client.bfs_max_depth == orig.opcua->client.bfs_max_depth);
    REQUIRE(rb.opcua->pipe.buffer_size     == orig.opcua->pipe.buffer_size);
    REQUIRE(rb.opcua->triggers_enabled     == orig.opcua->triggers_enabled);
    REQUIRE(rb.opcua->triggers.size()      == orig.opcua->triggers.size());

    // Verify individual trigger field values survive the write→read cycle.
    const auto& origT = orig.opcua->triggers.at("Chamber Oxygen Level");
    const auto& rbT   = rb.opcua->triggers.at("Chamber Oxygen Level");
    REQUIRE(rbT.signal       == origT.signal);
    REQUIRE(rbT.subsystem    == origT.subsystem);
    REQUIRE(rbT.rule_enabled == origT.rule_enabled);
    REQUIRE(rbT.start_value  == origT.start_value);
    REQUIRE(rbT.stop_value   == origT.stop_value);

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// RoundtripOpcuaFromScratch
// Build an OpcuaConfig in-memory (not from a fixture), write it, read it back.
// ---------------------------------------------------------------------------
TEST_CASE("RoundtripOpcuaFromScratch") {
    auto out = tmpPath("opcua_scratch");
    auto cfg = makeMinimalConfig();

    OpcuaConfig opcua;
    opcua.client.server_url        = "opc.tcp://localhost:4840";
    opcua.client.auth_mode         = "Anonymous";
    opcua.client.security_mode     = "None";
    opcua.client.security_policy   = "None";
    opcua.client.bfs_max_depth     = 8;
    opcua.client.publish_interval  = 500;
    opcua.client.sampling_interval = 500;
    opcua.client.session_timeout   = 30000;
    opcua.pipe.pipe_enabled        = true;
    opcua.pipe.buffer_size         = 32768;
    opcua.triggers_enabled         = true;

    OpcuaTrigger t;
    t.id           = "trigger_1";
    t.signal       = "test_signal";
    t.subsystem    = "TestSys";
    t.rule_enabled = true;
    t.start_value  = "0";
    t.stop_value   = "100";
    opcua.triggers["TestTrigger"] = std::move(t);
    cfg.opcua = std::move(opcua);

    REQUIRE_NOTHROW(MachineConfigWriter{cfg}.write(out));

    MachineConfigReader back{out};
    auto rb = back.parse();

    REQUIRE(rb.opcua.has_value());
    REQUIRE(rb.opcua->client.server_url      == "opc.tcp://localhost:4840");
    REQUIRE(rb.opcua->client.session_timeout == 30000);
    REQUIRE(rb.opcua->client.bfs_max_depth   == 8);
    REQUIRE(rb.opcua->pipe.buffer_size       == 32768);
    REQUIRE(rb.opcua->pipe.pipe_enabled      == true);
    REQUIRE(rb.opcua->triggers_enabled       == std::optional<bool>{true});
    REQUIRE(rb.opcua->triggers.size()        == 1);

    const auto& rt = rb.opcua->triggers.at("TestTrigger");
    REQUIRE(rt.signal      == std::optional<std::string>{"test_signal"});
    REQUIRE(rt.subsystem   == std::optional<std::string>{"TestSys"});
    REQUIRE(rt.rule_enabled == std::optional<bool>{true});
    REQUIRE(rt.start_value  == std::optional<std::string>{"0"});
    REQUIRE(rt.stop_value   == std::optional<std::string>{"100"});

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// WrittenHdf5ValidatesSchema
// JSON from reading back a written file has the required structure.
// ---------------------------------------------------------------------------
TEST_CASE("WrittenHdf5ValidatesSchema") {
    auto out = tmpPath("schema");
    MachineConfigReader src{REF};
    REQUIRE_NOTHROW(MachineConfigWriter{src.parse()}.write(out));

    auto j = nlohmann::json::parse(MachineConfigReader{out}.toJson());

    // Required top-level keys present with correct types
    REQUIRE(j.contains("meta"));
    REQUIRE(j.contains("machine"));
    REQUIRE(j.contains("optical_trains"));
    REQUIRE(j["meta"].is_object());
    REQUIRE(j["machine"].is_object());
    REQUIRE(j["optical_trains"].is_array());
    REQUIRE_FALSE(j["optical_trains"].empty());

    // Required meta fields
    REQUIRE(j["meta"]["schema_version"].get<std::string>() == "v1");
    REQUIRE_FALSE(j["meta"]["machine_name"].get<std::string>().empty());
    REQUIRE(j["meta"]["configuration_hash"].get<std::string>().size() == 64);

    // Required machine fields
    REQUIRE_FALSE(j["machine"]["manufacturer"].get<std::string>().empty());
    REQUIRE_FALSE(j["machine"]["model"].get<std::string>().empty());

    // Each train has required sub-objects
    for (const auto& train : j["optical_trains"]) {
        REQUIRE(train.contains("scanner"));
        REQUIRE(train.contains("light_source"));
        REQUIRE(train.contains("collimator"));
        REQUIRE(train.contains("scanner_card"));
        REQUIRE(train.contains("optional_components"));
        REQUIRE(train.contains("train_id"));
    }

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// BinaryRoundtripCorrectionGridHash  §4.15
// parseWithBinary + write; SHA-256 of copy's correction grid must match original.
// ---------------------------------------------------------------------------
TEST_CASE("BinaryRoundtripCorrectionGridHash") {
    auto out = tmpPath("bin_hash");
    MachineConfigWriter{MachineConfigReader{REF}.parseWithBinary()}.write(out);

    auto orig = MachineConfigReader{REF}.getCorrectionData(0);
    auto copy = MachineConfigReader{out}.getCorrectionData(0);
    REQUIRE(copy.data.size() == orig.data.size());

    auto hashOf = [](const CorrectionData& cd) {
        const auto* p = reinterpret_cast<const unsigned char*>(cd.data.data());
        std::vector<unsigned char> h;
        picosha2::hash256(p, p + cd.data.size() * sizeof(double), h);
        return h;
    };
    REQUIRE(hashOf(copy) == hashOf(orig));

    std::filesystem::remove(out);
}

// ---------------------------------------------------------------------------
// BinaryRoundtripFc3Size  §4.15
// Raw fc3 byte count of copy must match original.
// ---------------------------------------------------------------------------
TEST_CASE("BinaryRoundtripFc3Size") {
    auto out = tmpPath("fc3_size");
    MachineConfigWriter{MachineConfigReader{REF}.parseWithBinary()}.write(out);

    auto orig_bytes = MachineConfigReader{REF}.getScanFieldCorrectionBytes(0);
    auto copy_bytes = MachineConfigReader{out}.getScanFieldCorrectionBytes(0);
    REQUIRE(copy_bytes.size() == orig_bytes.size());

    std::filesystem::remove(out);
}

TEST_CASE("WriterRejectsUnknownFileVersion") {
    auto cfg = makeMinimalConfig();
    cfg.meta.file_version = "2.0";
    auto out = tmpPath("unknown_version");
    try {
        MachineConfigWriter{cfg}.write(out);
        FAIL("expected write() to throw for File_Version 2.0");
    } catch (const std::runtime_error& e) {
        REQUIRE(std::string(e.what()).find("2.0") != std::string::npos);
    }
    std::filesystem::remove(out);
}
