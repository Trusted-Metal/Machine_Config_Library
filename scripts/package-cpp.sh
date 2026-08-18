#!/usr/bin/env bash
# Packages the C++ "thin", find_package-based package (VALIDATION_PLAN.md
# §10; see docs/validation/cpp/tarball/results.md for the full design and
# why it isn't a self-contained tarball). Called by .releaserc.json's
# publishCmd, AFTER sync-version.mjs (run in prepareCmd) has already bumped
# cpp/CMakeLists.txt and cpp/cmake/Version.cmake to the real release
# version — configuring here, not reusing an earlier pre-bump configure, is
# what makes the packaged version.hpp/Config files carry the correct,
# just-bumped version.
#
# No failure-guarding: a broken C++ package should block the release the
# same way a broken Node/Python package already does today (see this
# script's caller in .releaserc.json — plain `&&`, no `|| true` anywhere) —
# fail loudly, fix it, try again, rather than silently ship a partial
# release.
set -euo pipefail

cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH="${HDF5_INSTALL:?HDF5_INSTALL must be set}" \
  -DHDF5_ROOT="${HDF5_INSTALL}" \
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5

rm -rf cpp/pkg-install
cmake --install cpp/build --config Release --prefix cpp/pkg-install

# Read the version from package.json directly, the same way sync-version.mjs
# itself does — not semantic-release's ${nextRelease.version} templating —
# for consistency with how nodejs/python's own publishCmd steps work (npm
# pack / python -m build also just read the already-bumped manifest).
version=$(node -p "require('./package.json').version")
tar -czf "cpp/machine-config-cpp-${version}.tar.gz" -C cpp/pkg-install .
echo "Packaged cpp/machine-config-cpp-${version}.tar.gz"
