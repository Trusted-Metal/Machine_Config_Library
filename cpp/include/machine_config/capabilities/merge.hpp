#pragma once
#include "machine_config/capabilities/generated.hpp"
#include "machine_config/models.hpp"

namespace machine_config::capabilities {

// Merge: overlay non-null keys from incoming onto a JSON clone of current.
// Replace: return incoming as-is (including `extra`).
template <typename T>
inline T applySetModeT(const T& current, const T& incoming, SetMode mode) {
  if (mode == SetMode::Replace) return incoming;
  nlohmann::json out = current;
  nlohmann::json inc = incoming;
  if (!inc.is_object()) return incoming;
  for (auto it = inc.begin(); it != inc.end(); ++it) {
    if (!it.value().is_null()) {
      out[it.key()] = it.value();
    }
  }
  return out.get<T>();
}

}  // namespace machine_config::capabilities
