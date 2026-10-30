# Full, untruncated version string — kept in sync with the repo root
# package.json by scripts/sync-version.mjs. May include a semver
# prerelease/build suffix (e.g. "0.2.0-rc.4") that project()'s own VERSION
# field in CMakeLists.txt structurally cannot hold. See
# include/machine_config/version.hpp.in for why this exists as a separate
# file/variable rather than just reusing PROJECT_VERSION.
set(MACHINE_CONFIG_FULL_VERSION "0.2.0-rc.4")
