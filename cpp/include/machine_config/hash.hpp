#pragma once
// SHA-256 configuration-integrity hash for MachineConfig.
//
// Computed over the canonical, include_binary=false JSON shape of the whole
// config (the same shape to_json()/export-json produce), minus
// meta.configuration_hash (can't hash itself) and meta.export_date (changes
// on every re-export even when nothing configuration-wise changed). Object
// keys are recursively sorted; arrays keep their existing element order.
//
// This hash is computed for MCF's own self-consistency across MCF's own
// five language implementations — it is NOT designed to match any other
// producer's own hashing scheme. A file MCF's own writer produced will read
// back valid; a file authored by anything else will very likely read back
// invalid — that's expected, not a bug: it answers "was this MCF-touched
// file tampered with since MCF itself last touched it," not "does this
// match some other system's proprietary algorithm."
//
// Binary raw_bytes of the scan-field correction file is always excluded regardless of whether the
// MachineConfig passed in happens to carry it (e.g. a config obtained via parseWithBinary()).
// ClearBox's correction grids are ordinary extension datasets and enter the hash through the
// extension digest.
//
// # Why this is the simplest of the five languages' implementations
//
// - Key sorting is free: to_json(json&, const MachineConfig&) (models.hpp)
//   already builds a plain nlohmann::json (not ordered_json — confirmed not
//   enabled anywhere in this project), whose object type is a sorted
//   std::map, so .dump() already emits sorted keys.
// - No float-rendering fixup is needed: verified empirically (a throwaway
//   standalone check: nlohmann::json(250.0).dump() -> "250.0", not "250")
//   that nlohmann::json's own double formatting already forces a trailing
//   ".0" on a whole-valued double, matching Rust's serde_json (and unlike
//   Node.js/Go, which both need a hand-rolled numeric renderer to get this).
// - is_valid needs no active exclusion at all: models.hpp's hand-written
//   to_json(json&, const MachineConfigMeta&) is an explicit initializer
//   list that simply never mentions is_valid, the same passive-exclusion
//   pattern already used for facility_id/config_author — there is nothing
//   to actively strip, unlike every other language's hash.
// - Scanner's four Invert_* flags need no special handling either: their
//   to_json already omits each key unless true (`if (s.invert_actual_x)
//   j["invert_actual_x"] = true;`), and this hash's first step genuinely is
//   `nlohmann::json j = config;` (the real to_json ADL call) — so the
//   omission applies before this header ever touches the json tree, with no
//   equivalent to the bug Node.js's hash hit in Step 3 (whose hash bypassed
//   JSON.stringify, and with it JSON.stringify's own equivalent replacer).
//
// SHA-256 is vendored inline below (detail::Sha256) rather than depended on
// via picosha2 — machine_config's own INTERFACE target does not expose
// picosha2 to its consumers today (only machine_config_cli/
// machine_config_tests get its include dir; see CMakeLists.txt), and this
// header should not change that packaging story just for one hash.

#include "machine_config/extension_contract.hpp"
#include "machine_config/models.hpp"
#include "machine_config/sha256.hpp"

#include <nlohmann/json.hpp>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <iomanip>
#include <sstream>
#include <string>

namespace machine_config {

namespace detail {

// Removes the binary correction-grid keys from j (a to_json(MachineConfig)
// result) regardless of whether they were populated — mirrors Python's
// _config_to_dict(config, include_binary=False), which always builds the
// non-binary shape irrespective of what the input carries.
inline void stripBinaryFieldsForHash(nlohmann::json& j) {
    if (!j.contains("optical_trains") || !j["optical_trains"].is_array()) return;
    for (auto& train : j["optical_trains"]) {
        if (train.contains("scan_field_correction_file") &&
            !train["scan_field_correction_file"].is_null()) {
            train["scan_field_correction_file"].erase("raw_bytes");
        }
    }
}

}  // namespace detail

/// Returns the exact canonical JSON text that computeConfigurationHash
/// hashes — exposed (not just an internal detail) so this can be diffed
/// directly against the other languages' equivalents when tracking down a
/// cross-language hash mismatch.
inline std::string canonicalizeForHash(const MachineConfig& config) {
    nlohmann::json j = config;  // invokes to_json(json&, const MachineConfig&)
    // Extension content enters the hash as one canonical digest per extension; configs without
    // extensions hash exactly as before.
    if (!config.extensions.empty()) {
        nlohmann::json digests = nlohmann::json::object();
        for (const auto& [name, tree] : config.extensions)
            digests[name] = extension::canonicalDigest(tree);
        j["extensions"] = std::move(digests);
    }
    detail::stripBinaryFieldsForHash(j);
    j["meta"].erase("configuration_hash");
    j["meta"].erase("export_date");
    return j.dump();  // plain nlohmann::json: sorted keys, compact by default
}

/// Returns the SHA-256 hex digest of config's canonical, hash-relevant content.
inline std::string computeConfigurationHash(const MachineConfig& config) {
    return detail::Sha256::hexDigest(canonicalizeForHash(config));
}

}  // namespace machine_config
