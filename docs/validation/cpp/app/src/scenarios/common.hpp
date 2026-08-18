#pragma once
// Shared types/helpers for validation scenarios.
// Result matches VALIDATION_PLAN.md §4's C++ entry-point contract exactly.

#include <picosha2.h>

#include <atomic>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <string>
#include <vector>

namespace scenarios {

struct Result {
    bool passed;
    std::string detail;
};

// Bitwise (NaN-aware) equality for correction-grid data — mirrors Rust's/
// Node's helper. Plain == on doubles would treat any NaN as unequal to
// itself, which is wrong for correction grids where NaN marks an
// intentional out-of-field cell.
inline bool bitwiseEqual(const std::vector<double>& a, const std::vector<double>& b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); ++i) {
        std::uint64_t ai, bi;
        std::memcpy(&ai, &a[i], sizeof(double));
        std::memcpy(&bi, &b[i], sizeof(double));
        if (ai != bi) return false;
    }
    return true;
}

// SHA-256 of the flat little-endian double buffer — same
// reinterpret_cast<const uint8_t*> idiom cpp/src/main.cpp's `correction-hash`
// CLI command already uses, so this matches real, established usage rather
// than inventing a new hashing convention.
inline std::string sha256Hex(const std::vector<double>& data) {
    const auto* p = reinterpret_cast<const std::uint8_t*>(data.data());
    return picosha2::hash256_hex_string(p, p + data.size() * sizeof(double));
}

// The shared, language-agnostic AV fixtures live at
// docs/validation/fixtures/<name> relative to the repo root, one level up
// from fixturesDir — same convention already used by every other language's
// AV scenario files.
inline std::filesystem::path avFixture(const std::filesystem::path& fixturesDir, const std::string& name) {
    // (fixturesDir / "..").lexically_normal() instead of fixturesDir.parent_path():
    // when fixturesDir carries a trailing separator (as it does when invoked from
    // CI/locally with ".../fixtures/"), parent_path() returns fixturesDir
    // unchanged instead of its parent — the trailing separator makes the final
    // path component empty, so there is nothing for parent_path() to "drop".
    // Appending ".." and normalizing lexically resolves it correctly regardless
    // of a trailing separator (same bug, same fix, already applied to Go's
    // filepath.Dir() equivalent — see VALIDATION_PLAN.md §9.4).
    return (fixturesDir / "..").lexically_normal() / "docs" / "validation" / "fixtures" / name;
}

// A unique path in the OS temp dir. Not an open handle (HighFive's File
// creates/truncates the path itself) — just a name nothing else will collide
// with within this process.
inline std::filesystem::path makeTempPath(const std::string& suffix) {
    static std::atomic<unsigned> counter{0};
    auto n = counter.fetch_add(1, std::memory_order_relaxed);
    return std::filesystem::temp_directory_path() /
        ("mcl_cpp_validation_" + std::to_string(n) + suffix);
}

} // namespace scenarios
