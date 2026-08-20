#pragma once
// S-01-09/AV-01-08 scenario declarations for the C++ validation app
// (VALIDATION_PLAN.md §8). AV-09-11 live in cpp/tests/ instead — see
// docs/validation/cpp/results.md for why.
//
// Each scenario keeps its own namespace (sXX/avXX), matching the original
// per-file layout exactly — main.cpp's `s01::run`, `av05::run`, etc. calls
// are unaffected by this file's consolidation.
#include "common.hpp"

namespace s01 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s02 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s03 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s04 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s05 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s06 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s07 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s08 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace s09 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av01 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av02 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av03 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av04 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av05 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av06 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av07 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av08 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av12 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
namespace av13 { scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir); }
