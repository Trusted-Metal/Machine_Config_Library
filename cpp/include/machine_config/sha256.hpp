#pragma once
// Minimal, self-contained SHA-256 (FIPS 180-4), shared by the configuration hash (hash.hpp) and the
// extension contract's canonical digest (extension_contract.hpp). Vendored inline rather than
// depending on picosha2: machine_config's INTERFACE target does not expose picosha2 to consumers.

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

}  // namespace detail

}  // namespace machine_config
