// Architectural guard: no File_Version adapter may import or call into
// another version's adapter code, even for logic that is byte-identical
// between versions today. If a later version depended on an earlier one's
// code, the earlier version could never be changed or removed without
// checking every dependent later version — and each additional version
// compounds the problem. This was already enforced for Python's real v1.1
// adapter and its test-only mock; this test enforces the same rule for
// C++'s version-scoped adapter files, including the test-only mock.
//
// Mechanism: read each relevant file's raw text and scan for substrings
// matching the shape `v<digits>_<digits>` (e.g. "v1_0", "v1_1"), case
// insensitively — this also catches per-version class names like
// "Hdf5AdapterV1_0" and "Hdf5WriterV1_0", not just namespace/include-path
// tokens. Each file's "own" version is derived from its location (the
// `vX_Y` directory it lives under, or — for the test-only mock, which has
// no directory of its own — its filename). Any *other* version token found
// in the file is a violation.
//
// Currently a no-op tripwire in the sense that it should already pass:
// only v1_0 is a real adapter today, and the mock at cpp/tests/mock_v1_1.hpp
// was made fully self-contained in the same change that added this test.
// It starts actually enforcing the moment a real v1_1 (or later) C++
// adapter is added and someone reaches back into v1_0's headers/namespace.
#include <catch2/catch_test_macros.hpp>

#include <filesystem>
#include <fstream>
#include <regex>
#include <sstream>
#include <string>
#include <vector>

#ifndef CPP_SOURCE_DIR
#  error "CPP_SOURCE_DIR must be defined by tests/CMakeLists.txt"
#endif

namespace {

namespace fs = std::filesystem;

std::string readFileText(const fs::path& p) {
    std::ifstream in(p, std::ios::binary);
    REQUIRE(in.is_open());
    std::ostringstream ss;
    ss << in.rdbuf();
    return ss.str();
}

// Every "v<digits>_<digits>" token in `text`, lower-cased, e.g. both
// "capabilities::v1_0" and "Hdf5AdapterV1_0" yield "v1_0".
std::vector<std::string> findVersionTokens(const std::string& text) {
    static const std::regex re(R"([vV](\d+)_(\d+))");
    std::vector<std::string> out;
    for (auto it = std::sregex_iterator(text.begin(), text.end(), re);
         it != std::sregex_iterator(); ++it) {
        out.push_back("v" + (*it)[1].str() + "_" + (*it)[2].str());
    }
    return out;
}

struct CheckedFile {
    fs::path path;
    std::string ownVersion;
};

void collectFilesUnder(const fs::path& dir, const std::string& ownVersion,
                        std::vector<CheckedFile>& out) {
    if (!fs::exists(dir)) return;
    for (const auto& entry : fs::recursive_directory_iterator(dir)) {
        if (entry.is_regular_file())
            out.push_back({entry.path(), ownVersion});
    }
}

std::vector<CheckedFile> collectCheckedFiles() {
    std::vector<CheckedFile> files;

    fs::path capsDir = fs::path(CPP_SOURCE_DIR) / "include/machine_config/capabilities";
    collectFilesUnder(capsDir / "v1_0", "v1_0", files);
    collectFilesUnder(capsDir / "v1_1", "v1_1", files); // no-op today; enforces once added

    fs::path mockPath = fs::path(CPP_SOURCE_DIR) / "tests/mock_v1_1.hpp";
    if (fs::exists(mockPath))
        files.push_back({mockPath, "v1_1"});

    return files;
}

} // namespace

TEST_CASE("version adapter isolation: no cross-version references") {
    auto files = collectCheckedFiles();

    // Sanity check: the guard isn't vacuously trivial — the real v1.0
    // adapter directory and the mock must both actually be found.
    bool sawV1_0 = false, sawMock = false;
    for (const auto& f : files) {
        if (f.ownVersion == "v1_0") sawV1_0 = true;
        if (f.path.filename() == "mock_v1_1.hpp") sawMock = true;
    }
    REQUIRE(sawV1_0);
    REQUIRE(sawMock);

    std::vector<std::string> violations;
    for (const auto& f : files) {
        std::string text = readFileText(f.path);
        for (const auto& token : findVersionTokens(text)) {
            if (token != f.ownVersion) {
                violations.push_back(
                    f.path.string() + ": found foreign version token '" + token +
                    "' (expected only '" + f.ownVersion + "')");
            }
        }
    }

    std::ostringstream msg;
    msg << "Cross-version references found:\n";
    for (const auto& v : violations) msg << "  " << v << "\n";
    INFO(msg.str());
    REQUIRE(violations.empty());
}
