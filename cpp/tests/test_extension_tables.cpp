// Compound tables (extension contract 0.1.1): validation, canonical digest, JSON form, HDF5 round
// trips through core, fixtures written by Node/Rust, and unsupported member shapes.
#include <catch2/catch_test_macros.hpp>

#include "machine_config/capabilities/v1_1/hdf5.hpp"
#include "machine_config/capabilities/v1_1/writer.hpp"
#include "machine_config/extension_contract.hpp"
#include "machine_config/extensions_hdf5.hpp"

#include <filesystem>
#include <fstream>
#include <string>

using namespace machine_config;
namespace ext = machine_config::extension;

namespace {

// Same tree and digest as nodejs/tests/extension_tables.test.ts and extension-contract's tests.
const std::string TABLE_DIGEST = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858";
const std::string LEGACY_DIGEST = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890";
const std::string TABLE_FIXTURES = FIXTURES_DIR;

ext::Table pointsTable() {
    ext::Table t;
    t.fields = {{"input_value", "f64", 0}, {"output_value", "f64", 0}};
    t.rows = 2;
    t.columns["input_value"] = std::vector<double>{4, 20};
    t.columns["output_value"] = std::vector<double>{-1, 6};
    return t;
}

ext::Tree tableTree() {
    ext::Tree tree;
    tree.attrs["Schema"] = {"utf8", std::string("1.0")};

    ext::Table constants;
    constants.fields = {{"name", "utf8", 64}, {"value", "f64", 0}};
    constants.rows = 2;
    constants.columns["name"] = std::vector<std::string>{"a", "b"};
    constants.columns["value"] = std::vector<double>{0.4375, -2.75};
    constants.attrs["Units"] = {"utf8", std::string("V")};
    tree.tables["Constants"] = constants;

    ext::Table mixed;
    mixed.fields = {{"id", "u32", 0}, {"count", "i64", 0}, {"ratio", "f32", 0}, {"flag", "u8", 0}, {"label", "utf8", 8}};
    mixed.rows = 3;
    mixed.columns["id"] = std::vector<std::uint64_t>{1, 2, 4294967295ULL};
    mixed.columns["count"] = std::vector<std::int64_t>{-9007199254740993LL, 0, 7};
    mixed.columns["ratio"] = std::vector<double>{0.5, -1.5, 2.0};
    mixed.columns["flag"] = std::vector<std::uint64_t>{0, 1, 1};
    mixed.columns["label"] = std::vector<std::string>{"x", "\xC2\xB5m", ""};
    tree.tables["Mixed"] = mixed;

    ext::Table empty;
    empty.fields = {{"input_value", "f64", 0}, {"output_value", "f64", 0}};
    empty.rows = 0;
    empty.columns["input_value"] = std::vector<double>{};
    empty.columns["output_value"] = std::vector<double>{};
    tree.tables["Empty"] = empty;

    ext::Tree child;
    child.tables["Points"] = pointsTable();
    tree.children["Train_1"] = child;
    return tree;
}

nlohmann::json readJson(const std::string& name) {
    std::ifstream in(TABLE_FIXTURES + "/" + name);
    REQUIRE(in.good());
    return nlohmann::json::parse(in);
}

std::string tableTmp(const std::string& name) {
    return (std::filesystem::temp_directory_path() / ("mcf_ext_tables_" + name + ".h5")).string();
}

MachineConfig v1_1Config() {
    capabilities::v1_1::Hdf5AdapterV1_1 adapter(TABLE_FIXTURES + "/reference_config_v1_1.h5");
    return adapter.parse();
}

std::string validationMessage(const ext::Tree& t) {
    try {
        ext::validate(t, "Ext");
    } catch (const ext::ContractError& e) {
        return e.what();
    }
    return "";
}

}  // namespace

TEST_CASE("ExtensionTablesContractStaysOnThe01Line") {
    REQUIRE(std::string(ext::CONTRACT_VERSION).rfind("0.1.", 0) == 0);
}

TEST_CASE("ExtensionTablesDigestIsPinnedAndMatchesTheOtherLanguages") {
    REQUIRE(ext::canonicalDigest(tableTree()) == TABLE_DIGEST);
}

TEST_CASE("ExtensionTablesGoldenJsonFixturesKeepTheirDigests") {
    REQUIRE(ext::canonicalDigest(ext::treeFromJson(readJson("extension_sample_tree.json"))) == LEGACY_DIGEST);
    REQUIRE(ext::canonicalDigest(ext::treeFromJson(readJson("extension_table_sample_tree.json"))) == TABLE_DIGEST);
}

TEST_CASE("ExtensionTablesEmptyMapDoesNotChangeTheDigestOrJson") {
    ext::Tree t;
    t.attrs["A"] = {"u32", std::uint64_t{1}};
    const auto plain = ext::canonicalDigest(t);
    t.tables.clear();
    REQUIRE(ext::canonicalDigest(t) == plain);
    REQUIRE_FALSE(ext::treeToJson(t).contains("tables"));
}

TEST_CASE("ExtensionTablesJsonRoundTripIsLossless") {
    auto j = ext::treeToJson(tableTree());
    REQUIRE(j["tables"]["Mixed"]["columns"]["count"][0] == "-9007199254740993");
    REQUIRE(j["tables"]["Constants"]["fields"][0] ==
            nlohmann::json({{"name", "name"}, {"dtype", "utf8"}, {"size", 64}}));
    REQUIRE(j["tables"]["Constants"]["fields"][1] == nlohmann::json({{"name", "value"}, {"dtype", "f64"}}));
    auto back = ext::treeFromJson(nlohmann::json::parse(j.dump()));
    REQUIRE(ext::canonicalDigest(back) == TABLE_DIGEST);
}

TEST_CASE("ExtensionTablesDigestIsSensitiveToFieldOrderSizeValuesAndRows") {
    const auto base = ext::canonicalDigest(tableTree());

    auto swapped = tableTree();
    std::swap(swapped.tables["Constants"].fields[0], swapped.tables["Constants"].fields[1]);
    REQUIRE(ext::canonicalDigest(swapped) != base);

    auto wider = tableTree();
    wider.tables["Constants"].fields[0].size = 32;
    REQUIRE(ext::canonicalDigest(wider) != base);

    auto edited = tableTree();
    std::get<std::vector<double>>(edited.tables["Constants"].columns["value"])[1] = -2.5;
    REQUIRE(ext::canonicalDigest(edited) != base);

    auto shorter = tableTree();
    auto& p = shorter.children["Train_1"].tables["Points"];
    p.rows = 1;
    p.columns["input_value"] = std::vector<double>{4};
    p.columns["output_value"] = std::vector<double>{-1};
    REQUIRE(ext::canonicalDigest(shorter) != base);
}

TEST_CASE("ExtensionTablesValidationRejectsMalformedTablesWithAPath") {
    auto bad = [](auto mutate) {
        ext::Tree t;
        auto p = pointsTable();
        mutate(p);
        t.tables["T"] = p;
        return validationMessage(t);
    };
    REQUIRE(bad([](ext::Table& t) { t.fields.clear(); }).find("at least one field") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.fields[1].name = "input_value"; }).find("duplicate field") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.rows = 3; }).find("does not match 3 rows") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.columns.erase("output_value"); }).find("missing column") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.columns["extra"] = std::vector<double>{0, 0}; }).find("no matching field") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.fields[0].size = 8; }).find("only valid for utf8") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) {
                t.fields[0] = {"input_value", "utf8", 0};
                t.columns["input_value"] = std::vector<std::string>{"a", "b"};
            }).find("positive integer size") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) {
                t.fields[0] = {"input_value", "utf8", 2};
                t.columns["input_value"] = std::vector<std::string>{"abc", "b"};
            }).find("do not fit the 2-byte field") != std::string::npos);
    REQUIRE(bad([](ext::Table& t) { t.columns["output_value"] = std::vector<double>{0}; }).find("Ext/T/output_value") !=
            std::string::npos);

    ext::Tree clash;
    clash.children["T"] = ext::Tree{};
    clash.tables["T"] = pointsTable();
    REQUIRE(validationMessage(clash).find("cannot share a name") != std::string::npos);
}

TEST_CASE("ExtensionTablesRoundTripEveryTableVerbatim") {
    auto cfg = v1_1Config();
    cfg.extensions["Private_Ext"] = tableTree();
    auto path = tableTmp("roundtrip");
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);

    capabilities::v1_1::Hdf5AdapterV1_1 adapter(path);
    auto got = adapter.parse().extensions.at("Private_Ext");
    REQUIRE(ext::canonicalDigest(got) == TABLE_DIGEST);
    const auto& mixed = got.tables.at("Mixed");
    REQUIRE(mixed.fields[4].dtype == "utf8");
    REQUIRE(mixed.fields[4].size == 8);
    REQUIRE(std::get<std::vector<std::string>>(mixed.columns.at("label"))[1] == "\xC2\xB5m");
    REQUIRE(got.tables.at("Empty").rows == 0);
    REQUIRE(std::get<std::string>(got.tables.at("Constants").attrs.at("Units").value) == "V");
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionTablesATreeWithoutTablesHasNoneAfterARoundTrip") {
    auto cfg = v1_1Config();
    ext::Tree t;
    t.attrs["A"] = {"u32", std::uint64_t{1}};
    cfg.extensions["Private_Ext"] = t;
    auto path = tableTmp("plain");
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);
    capabilities::v1_1::Hdf5AdapterV1_1 adapter(path);
    REQUIRE(adapter.parse().extensions.at("Private_Ext").tables.empty());
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionTablesWriterRejectsAnInvalidTableBeforeCreatingTheFile") {
    auto cfg = v1_1Config();
    auto bad = tableTree();
    bad.tables["Constants"].columns["name"] = std::vector<std::string>{"only-one"};
    cfg.extensions["Bad"] = bad;
    auto path = tableTmp("never");
    std::filesystem::remove(path);
    REQUIRE_THROWS_AS(capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path), ext::ContractError);
    REQUIRE_FALSE(std::filesystem::exists(path));
}

TEST_CASE("ExtensionTablesReadFixturesWrittenByOtherLanguages") {
    for (const char* name : {"extension_table_sample_node_v1_1.h5", "extension_table_sample_rust_v1_1.h5"}) {
        capabilities::v1_1::Hdf5AdapterV1_1 adapter(TABLE_FIXTURES + "/" + name);
        auto cfg = adapter.parse();
        REQUIRE(cfg.extensions.count("Private_Ext") == 1);
        REQUIRE(ext::canonicalDigest(cfg.extensions.at("Private_Ext")) == TABLE_DIGEST);
    }
}

namespace {

/// Writes a config with an empty Private_Ext, then lets `add` create one raw dataset inside it.
template <typename Add>
std::string fileWithRawDataset(const std::string& name, Add add) {
    auto cfg = v1_1Config();
    cfg.extensions["Private_Ext"] = ext::Tree{};
    auto path = tableTmp(name);
    capabilities::v1_1::Hdf5WriterV1_1(cfg).write(path);
    hid_t file = H5Fopen(path.c_str(), H5F_ACC_RDWR, H5P_DEFAULT);
    REQUIRE(file >= 0);
    hid_t group = H5Gopen2(file, "Extensions/Private_Ext", H5P_DEFAULT);
    REQUIRE(group >= 0);
    add(group);
    H5Gclose(group);
    H5Fclose(file);
    return path;
}

std::string parseError(const std::string& path) {
    try {
        capabilities::v1_1::Hdf5AdapterV1_1 adapter(path);
        adapter.parse();
    } catch (const ext::ContractError& e) {
        return e.what();
    }
    return "";
}

}  // namespace

TEST_CASE("ExtensionTablesRejectsACompoundDatasetThatIsNot1D") {
    auto path = fileWithRawDataset("rank2", [](hid_t g) {
        hid_t tid = H5Tcreate(H5T_COMPOUND, 8);
        H5Tinsert(tid, "a", 0, H5T_NATIVE_DOUBLE);
        hsize_t dims[2] = {2, 2};
        hid_t space = H5Screate_simple(2, dims, nullptr);
        hid_t ds = H5Dcreate2(g, "Grid", tid, space, H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT);
        H5Dclose(ds);
        H5Sclose(space);
        H5Tclose(tid);
    });
    const auto message = parseError(path);
    REQUIRE(message.find("Private_Ext/Grid") != std::string::npos);
    REQUIRE(message.find("1-D") != std::string::npos);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionTablesRejectsAnArrayMember") {
    auto path = fileWithRawDataset("array", [](hid_t g) {
        hsize_t adims[1] = {2};
        hid_t arr = H5Tarray_create2(H5T_NATIVE_DOUBLE, 1, adims);
        hid_t tid = H5Tcreate(H5T_COMPOUND, 16);
        H5Tinsert(tid, "a", 0, arr);
        hsize_t dim = 1;
        hid_t space = H5Screate_simple(1, &dim, nullptr);
        hid_t ds = H5Dcreate2(g, "Arr", tid, space, H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT);
        H5Dclose(ds);
        H5Sclose(space);
        H5Tclose(tid);
        H5Tclose(arr);
    });
    REQUIRE(parseError(path).find("Private_Ext/Arr/a") != std::string::npos);
    std::filesystem::remove(path);
}

TEST_CASE("ExtensionTablesRejectsAVariableLengthStringMember") {
    auto path = fileWithRawDataset("vlen", [](hid_t g) {
        hid_t st = H5Tcopy(H5T_C_S1);
        H5Tset_size(st, H5T_VARIABLE);
        hid_t tid = H5Tcreate(H5T_COMPOUND, sizeof(char*));
        H5Tinsert(tid, "s", 0, st);
        hsize_t dim = 1;
        hid_t space = H5Screate_simple(1, &dim, nullptr);
        hid_t ds = H5Dcreate2(g, "Vl", tid, space, H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT);
        H5Dclose(ds);
        H5Sclose(space);
        H5Tclose(tid);
        H5Tclose(st);
    });
    const auto message = parseError(path);
    REQUIRE(message.find("Private_Ext/Vl/s") != std::string::npos);
    REQUIRE(message.find("variable-length") != std::string::npos);
    std::filesystem::remove(path);
}
