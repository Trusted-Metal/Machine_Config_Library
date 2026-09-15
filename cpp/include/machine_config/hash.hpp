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
// Binary correction-grid data (correction_data/inverse_correction_data/
// raw_bytes) is always excluded regardless of whether the MachineConfig
// passed in happens to carry it (e.g. a config obtained via
// parseWithBinary()) — a separate, dedicated mechanism (correction-hash)
// already exists for binary-grid integrity.
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

#include "machine_config/models.hpp"

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

// Minimal, self-contained SHA-256 (FIPS 180-4). Not constant-time; this is
// a content-integrity fingerprint, not a secret-handling primitive.
class Sha256 {
public:
    static std::string hexDigest(const std::string& message) {
        Sha256 ctx;
        ctx.update(reinterpret_cast<const std::uint8_t*>(message.data()), message.size());
        return ctx.finalizeHex();
    }

private:
    std::array<std::uint32_t, 8> h_{
        0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
        0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u,
    };
    std::array<std::uint8_t, 64> buffer_{};
    std::size_t bufferLen_ = 0;
    std::uint64_t totalLen_ = 0;

    static constexpr std::array<std::uint32_t, 64> kK = {
        0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u,
        0x923f82a4u, 0xab1c5ed5u, 0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
        0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u, 0xe49b69c1u, 0xefbe4786u,
        0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
        0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u,
        0x06ca6351u, 0x14292967u, 0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
        0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u, 0xa2bfe8a1u, 0xa81a664bu,
        0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
        0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au,
        0x5b9cca4fu, 0x682e6ff3u, 0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
        0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u,
    };

    static std::uint32_t rotr(std::uint32_t x, int n) {
        return (x >> n) | (x << (32 - n));
    }

    void processBlock(const std::uint8_t* block) {
        std::array<std::uint32_t, 64> w{};
        for (int i = 0; i < 16; ++i) {
            w[i] = (static_cast<std::uint32_t>(block[i * 4]) << 24) |
                   (static_cast<std::uint32_t>(block[i * 4 + 1]) << 16) |
                   (static_cast<std::uint32_t>(block[i * 4 + 2]) << 8) |
                   (static_cast<std::uint32_t>(block[i * 4 + 3]));
        }
        for (int i = 16; i < 64; ++i) {
            std::uint32_t s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            std::uint32_t s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }

        std::uint32_t a = h_[0], b = h_[1], c = h_[2], d = h_[3];
        std::uint32_t e = h_[4], f = h_[5], g = h_[6], hh = h_[7];

        for (int i = 0; i < 64; ++i) {
            std::uint32_t s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
            std::uint32_t ch = (e & f) ^ ((~e) & g);
            std::uint32_t temp1 = hh + s1 + ch + kK[i] + w[i];
            std::uint32_t s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
            std::uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            std::uint32_t temp2 = s0 + maj;

            hh = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = b;
            b = a;
            a = temp1 + temp2;
        }

        h_[0] += a; h_[1] += b; h_[2] += c; h_[3] += d;
        h_[4] += e; h_[5] += f; h_[6] += g; h_[7] += hh;
    }

    void update(const std::uint8_t* data, std::size_t len) {
        totalLen_ += len;
        while (len > 0) {
            std::size_t take = std::min(len, buffer_.size() - bufferLen_);
            std::memcpy(buffer_.data() + bufferLen_, data, take);
            bufferLen_ += take;
            data += take;
            len -= take;
            if (bufferLen_ == buffer_.size()) {
                processBlock(buffer_.data());
                bufferLen_ = 0;
            }
        }
    }

    std::string finalizeHex() {
        std::uint64_t bitLen = totalLen_ * 8;
        std::uint8_t pad = 0x80;
        update(&pad, 1);
        std::uint8_t zero = 0x00;
        while (bufferLen_ != 56) {
            update(&zero, 1);
        }
        std::array<std::uint8_t, 8> lenBytes{};
        for (int i = 0; i < 8; ++i) {
            lenBytes[7 - i] = static_cast<std::uint8_t>(bitLen >> (8 * i));
        }
        // Append the length directly (bypassing update()'s totalLen_
        // bookkeeping, which must not count these bytes) so this block is
        // processed as the final block.
        std::memcpy(buffer_.data() + bufferLen_, lenBytes.data(), 8);
        bufferLen_ += 8;
        processBlock(buffer_.data());
        bufferLen_ = 0;

        std::ostringstream oss;
        for (std::uint32_t word : h_) {
            oss << std::hex << std::setfill('0');
            for (int shift = 24; shift >= 0; shift -= 8) {
                oss << std::setw(2) << ((word >> shift) & 0xff);
            }
        }
        return oss.str();
    }
};

// Removes the binary correction-grid keys from j (a to_json(MachineConfig)
// result) regardless of whether they were populated — mirrors Python's
// _config_to_dict(config, include_binary=False), which always builds the
// non-binary shape irrespective of what the input carries.
inline void stripBinaryFieldsForHash(nlohmann::json& j) {
    if (!j.contains("optical_trains") || !j["optical_trains"].is_array()) return;
    for (auto& train : j["optical_trains"]) {
        if (train.contains("optional_components")) {
            auto& oc = train["optional_components"];
            if (oc.contains("clearbox") && !oc["clearbox"].is_null()) {
                oc["clearbox"].erase("correction_data");
                oc["clearbox"].erase("inverse_correction_data");
            }
        }
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
