// Extension contract: validation, canonical digest, JSON form, HDF5 round trips, cross-language
// fixtures, and core integration (reader/writer/hash/session).
#include <catch2/catch_test_macros.hpp>

#include "machine_config/capabilities.hpp"
#include "machine_config/capabilities/v1_0/hdf5.hpp"
#include "machine_config/capabilities/v1_0/writer.hpp"
#include "machine_config/capabilities/v1_1/hdf5.hpp"
#include "machine_config/capabilities/v1_1/writer.hpp"
#include "machine_config/extension_contract.hpp"
#include "machine_config/extensions_hdf5.hpp"
#include "machine_config/hash.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#include <cmath>
#include <filesystem>
#include <fstream>
#include <sstream>

using namespace machine_config;
using machine_config::capabilities::MachineConfigFileV1_0;
using machine_config::capabilities::SetMode;
namespace ext = machine_config::extension;

namespace {

const std::string PINNED_DIGEST = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890";
const std::string FIXTURES = FIXTURES_DIR;

ext::Tree sampleTree() {
    ext::Tree t;
    t.attrs["Schema"] = {"utf8", std::string("1.2")};
    t.attrs["Flag"] = {"u32", std::uint64_t{1}};
    t.attrs["Float_Flag"] = {"f64", 1.0};
    t.attrs["Small"] = {"i8", std::int64_t{-5}};
    t.attrs["Wide"] = {"i64", std::int64_t{-9007199254740993LL}};
    t.attrs["Huge"] = {"u64", std::uint64_t{18446744073709551615ULL}};
    t.attrs["Ratio"] = {"f32", 0.5};
    t.attrs["Pair"] = {"f64", std::vector<double>{1.5, -2.5}};
    t.attrs["Ids"] = {"u32", std::vector<std::uint64_t>{1, 2, 3}};
    ext::Tree cond;
    cond.attrs["Value"] = {"utf8", std::string("running")};
    ext::Tree job;
    job.attrs["Label"] = {"utf8", std::string("Job Start")};
    job.children["Condition 1"] = cond;
    t.children["Job Start"] = job;
    ext::Dataset grid;
    grid.dtype = "f64";
    grid.shape = {2, 3};
    grid.data = std::vector<double>{1, 2, 3, 4, 5, 6.5};
    grid.attrs["Units"] = {"utf8", std::string("mm")};
    t.datasets["Grid"] = grid;
    return t;
}

std::string tmp(const std::string& name) {
    return (std::filesystem::temp_directory_path() / ("mcf_ext_" + name + ".h5")).string();
}

MachineConfig readV1_1Fixture() {
    capabilities::v1_1::Hdf5AdapterV1_1 adapter(FIXTURES + "/reference_config_v1_1.h5");
    return adapter.parse();
}

}  // namespace

TEST_CASE("ExtensionPinnedConformanceDigest") {
    REQUIRE(ext::canonicalDigest(sampleTree()) == PINNED_DIGEST);
}

TEST_CASE("ExtensionDigestTreatsNegativeZeroAndNaNCanonically") {
    auto mk = [](double v) {
        ext::Tree t;
        t.attrs["X"] = {"f64", v};
        return ext::canonicalDigest(t);
    };
    REQUIRE(mk(-0.0) == mk(0.0));
    REQUIRE(mk(std::nan("1")) == mk(std::nan("2")));
    REQUIRE(mk(1.0) != mk(2.0));
}

TEST_CASE("ExtensionGoldenJsonFixtureHasThePinnedDigest") {
    std::ifstream in(FIXTURES + "/extension_sample_tree.json");
    REQUIRE(in.good());
    auto j = nlohmann::json::parse(in);
    REQUIRE(ext::canonicalDigest(ext::treeFromJson(j)) == PINNED_DIGEST);
}

TEST_CASE("ExtensionJsonRoundTripIsLossless") {
    auto tree = sampleTree();
    tree.attrs["Weird"] = {"f64", std::vector<double>{std::nan(""), INFINITY, -INFINITY}};
    auto j = ext::treeToJson(tree);
    REQUIRE(j["attrs"]["Wide"]["value"] == "-9007199254740993");
    REQUIRE(j["attrs"]["Huge"]["value"] == "18446744073709551615");
    auto back = ext::treeFromJson(nlohmann::json::parse(j.dump()));
    REQUIRE(ext::canonicalDigest(back) == ext::canonicalDigest(tree));
}

TEST_CASE("ExtensionJsonRejectsOutOfRangeAndUnknownDtypes") {
    REQUIRE_THROWS_AS(ext::treeFromJson(nlohmann::json::parse(R"({"attrs":{"X":{"dtype":"u8","value":256}}})")),
                      ext::ContractError);
    REQUIRE_THROWS_AS(ext::treeFromJson(nlohmann::json::parse(R"({"attrs":{"X":{"dtype":"u128","value":1}}})")),
                      ext::ContractError);
    REQUIRE_THROWS_AS(ext::treeFromJson(nlohmann::json::parse(R"({"attrs":{"X":{"dtype":"i64","value":1}}})")),
                      ext::ContractError);
}

TEST_CASE("ExtensionValidationRejectsInvalidTreesWithAPath") {
    std::vector<ext::Tree> bad;
    {
        ext::Tree t; t.attrs["A"] = {"u8", std::uint64_t{300}}; bad.push_back(t);
    }
    {
        ext::Tree t; t.attrs["A"] = {"i32", std::string("x")}; bad.push_back(t);
    }
    {
        ext::Tree t; t.attrs["A"] = {"bogus", std::int64_t{1}}; bad.push_back(t);
    }
    {
        ext::Tree t; t.attrs[""] = {"u8", std::uint64_t{1}}; bad.push_back(t);
    }
    {
        ext::Tree t; t.children["a/b"] = ext::Tree{}; bad.push_back(t);
    }
    {
        ext::Tree t;
        t.children["X"] = ext::Tree{};
        ext::Dataset d; d.dtype = "f64"; d.shape = {1}; d.data = std::vector<double>{1};
        t.datasets["X"] = d;
        bad.push_back(t);
    }
    {
        ext::Tree t;
        ext::Dataset d; d.dtype = "f64"; d.shape = {2, 2}; d.data = std::vector<double>{1, 2, 3};
        t.datasets["D"] = d;
        bad.push_back(t);
    }
    for (const auto& t : bad) {
        try {
            ext::validate(t, "Ext");
            FAIL("expected a validation error");
        } catch (const ext::ContractError& e) {
            REQUIRE(e.path().rfind("Ext", 0) == 0);
        }
    }

    ext::Tree outer, inner;
    inner.attrs["A"] = {"u8", std::uint64_t{0}};
    inner.attrs["B"] = {"i8", std::int64_t{-200}};
    outer.children["Outer"] = inner;
    try {
        ext::validate(outer, "Ext");
        FAIL("expected a validation error");
    } catch (const ext::ContractError& e) {
        REQUIRE(e.path() == "Ext/Outer/@B");
    }
}

TEST_CASE("ExtensionHdf5HandlesAllDtypesAndShapes") {
    ext::Tree tree;
    tree.attrs["EmptyList"] = {"i32", std::vector<std::int64_t>{}};
    tree.attrs["Strings"] = {"utf8", std::vector<std::string>{"a", "b\xC3\xA9", ""}};
    tree.attrs["S_i8"] = {"i8", std::int64_t{-8}};
    tree.attrs["S_i16"] = {"i16", std::int64_t{-16}};
    tree.attrs["S_i32"] = {"i32", std::int64_t{-32}};
    tree.attrs["S_u8"] = {"u8", std::uint64_t{8}};
    tree.attrs["S_u16"] = {"u16", std::uint64_t{16}};
    tree.attrs["S_u32"] = {"u32", std::uint64_t{32}};
    tree.attrs["S_f32"] = {"f32", 0.25};
    ext::Dataset scalar; scalar.dtype = "i16"; scalar.data = std::vector<std::int64_t>{-3};
    tree.datasets["Scalar"] = scalar;
    ext::Dataset cube; cube.dtype = "u8"; cube.shape = {2, 2, 2};
    cube.data = std::vector<std::uint64_t>{1, 2, 3, 4, 5, 6, 7, 8};
    tree.datasets["Cube"] = cube;
    const auto want = ext::canonicalDigest(tree);

    auto path = tmp("io");
    {
        HighFive::File f(path, HighFive::File::ReadWrite | HighFive::File::Create | HighFive::File::Truncate);
        ext::writeExtensionTree(f.getId(), "T", tree);
    }
    ext::Tree back;
    {
        HighFive::File f(path, HighFive::File::ReadOnly);
        auto g = f.getGroup("T");
        back = ext::readExtensionTree(g.getId(), "T");
    }
    std::filesystem::remove(path);
    REQUIRE(ext::canonicalDigest(back) == want);
}

TEST_CASE("ExtensionOpcuaGroupIsAnOpaqueExtension") {
    MachineConfigReader plain{FIXTURES + "/reference_config.h5"};
    auto plainExt = plain.parse().extensions;
    REQUIRE(plainExt.size() == 1);
    REQUIRE(plainExt.count("ClearBox") == 1);

    MachineConfigReader reader{FIXTURES + "/reference_config_opcua.h5"};
    auto cfg = reader.parse();
    REQUIRE(cfg.extensions.size() == 2);
    REQUIRE(cfg.extensions.count("ClearBox") == 1);
    const auto& tree = cfg.extensions.at("TM_OPCUA");
    REQUIRE(tree.children.count("Client") == 1);
    REQUIRE(tree.children.count("Pipe") == 1);
    REQUIRE(tree.children.count("Triggers") == 1);
    const auto& url = tree.children.at("Client").attrs.at("Server_URL");
    REQUIRE(url.dtype == "utf8");
    REQUIRE(std::get<std::string>(url.value) ==
            "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer");
    const auto& trig = tree.children.at("Triggers");
    REQUIRE(trig.children.count("Laser Emission Interlock") == 1);
    REQUIRE(trig.children.count("Chamber Oxygen Level") == 1);
}

TEST_CASE("ExtensionToJsonCarriesExtensionsNotTypedOpcua") {
    MachineConfigReader reader{FIXTURES + "/reference_config_opcua.h5"};
    auto j = nlohmann::json::parse(reader.toJson());
    REQUIRE(j.contains("extensions"));
    REQUIRE(j["extensions"].contains("TM_OPCUA"));
    REQUIRE_FALSE(j.contains("opcua"));
}

TEST_CASE("ExtensionReadsFixturesWrittenByOtherLanguages") {
    for (const char* name : {"extension_sample_node_v1_1.h5", "extension_sample_rust_v1_1.h5"}) {
        capabilities::v1_1::Hdf5AdapterV1_1 adapter(FIXTURES + "/" + name);
        auto cfg = adapter.parse();
        REQUIRE(cfg.extensions.count("Private_Ext") == 1);
        REQUIRE(ext::canonicalDigest(cfg.extensions.at("Private_Ext")) == PINNED_DIGEST);
    }
}

TEST_CASE("ExtensionV1_1RoundTripPreservesDigest") {
    auto cfg = readV1_1Fixture();
    REQUIRE(cfg.extensions.count("ClearBox") == 1);
    cfg.extensions["Private_Ext"] = sampleTree();
    auto path = tmp("v11");
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);
    capabilities::v1_1::Hdf5AdapterV1_1 adapter(path);
    auto back = adapter.parse();
    REQUIRE(ext::canonicalDigest(back.extensions.at("Private_Ext")) == PINNED_DIGEST);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionWriterRejectsInvalidTreeBeforeCreatingTheFile") {
    auto cfg = readV1_1Fixture();
    ext::Tree bad;
    bad.attrs["A"] = {"u8", std::uint64_t{999}};
    cfg.extensions["Bad"] = bad;
    auto path = tmp("never");
    std::filesystem::remove(path);
    REQUIRE_THROWS_AS(capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path), ext::ContractError);
    REQUIRE_FALSE(std::filesystem::exists(path));
}

TEST_CASE("ExtensionWriterAcceptsAClearBoxExtensionTree") {
    auto cfg = readV1_1Fixture();
    ext::Tree tree;
    tree.attrs["Output_Path"] = {"utf8", std::string("x")};
    cfg.extensions["ClearBox"] = tree;
    auto path = tmp("clearbox_ext");
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);
    auto back = capabilities::v1_1::Hdf5AdapterV1_1(path).parse();
    REQUIRE(std::get<std::string>(back.extensions.at("ClearBox").attrs.at("Output_Path").value) == "x");
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionV1_0OpcuaGroupRoundTripsAsTmOpcua") {
    capabilities::v1_0::Hdf5AdapterV1_0 adapter(FIXTURES + "/reference_config_opcua.h5");
    auto cfg = adapter.parse();
    const auto want = ext::canonicalDigest(cfg.extensions.at("TM_OPCUA"));
    auto path = tmp("legacy");
    capabilities::v1_0::Hdf5WriterV1_0(cfg).write(path);
    {
        HighFive::File f(path, HighFive::File::ReadOnly);
        REQUIRE(f.exist("OPCUA"));
    }
    capabilities::v1_0::Hdf5AdapterV1_0 again(path);
    REQUIRE(ext::canonicalDigest(again.parse().extensions.at("TM_OPCUA")) == want);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionV1_0CannotHoldOtherExtensions") {
    capabilities::v1_0::Hdf5AdapterV1_0 adapter(FIXTURES + "/reference_config.h5");
    auto cfg = adapter.parse();
    cfg.extensions["Private_Ext"] = sampleTree();
    auto path = tmp("never_v10");
    std::filesystem::remove(path);
    REQUIRE_THROWS(capabilities::v1_0::Hdf5WriterV1_0(cfg).write(path));
    REQUIRE_FALSE(std::filesystem::exists(path));
}

TEST_CASE("ExtensionV1_0OpcuaGroupMigratesToAV1_1Extension") {
    capabilities::v1_0::Hdf5AdapterV1_0 adapter(FIXTURES + "/reference_config_opcua.h5");
    auto cfg = adapter.parse();
    const auto want = ext::canonicalDigest(cfg.extensions.at("TM_OPCUA"));
    cfg.meta.file_version = "1.1";
    auto path = tmp("migrated");
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);
    {
        HighFive::File f(path, HighFive::File::ReadOnly);
        REQUIRE(f.exist("Extensions/TM_OPCUA"));
    }
    capabilities::v1_1::Hdf5AdapterV1_1 again(path);
    REQUIRE(ext::canonicalDigest(again.parse().extensions.at("TM_OPCUA")) == want);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionSessionOpenEditSavePreservesExtensions") {
    capabilities::v1_0::Hdf5AdapterV1_0 adapter(FIXTURES + "/reference_config_opcua.h5");
    const auto want = ext::canonicalDigest(adapter.parse().extensions.at("TM_OPCUA"));

    auto opened = MachineConfigFileV1_0::open(FIXTURES + "/reference_config_opcua.h5");
    REQUIRE(opened.ok());
    auto scanner = opened.value()->getScanner(0);
    REQUIRE(scanner.ok());
    auto edited = scanner.value();
    edited.working_distance = 77.5;
    REQUIRE(opened.value()->setScanner(0, edited, SetMode::Merge).ok());
    auto path = tmp("session");
    std::string p = path;
    REQUIRE(opened.value()->save(&p).ok());
    opened.value()->close();

    capabilities::v1_0::Hdf5AdapterV1_0 again(path);
    REQUIRE(ext::canonicalDigest(again.parse().extensions.at("TM_OPCUA")) == want);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionHashUnchangedWithoutExtensionsAndChangesWithThem") {
    auto cfg = readV1_1Fixture();
    const auto base = computeConfigurationHash(cfg);

    cfg.extensions["Private_Ext"] = sampleTree();
    const auto withExt = computeConfigurationHash(cfg);
    REQUIRE(withExt != base);

    cfg.extensions["Private_Ext"].attrs["Flag"] = {"u32", std::uint64_t{2}};
    REQUIRE(computeConfigurationHash(cfg) != withExt);

    cfg.extensions.erase("Private_Ext");
    REQUIRE(computeConfigurationHash(cfg) == base);
}
