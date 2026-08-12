#pragma once
#include <string>

namespace machine_config::capabilities {

struct CapabilityError {
  std::string code;
  std::string message;
};

inline CapabilityError capabilityError(std::string code, std::string message) {
  return CapabilityError{std::move(code), std::move(message)};
}

}  // namespace machine_config::capabilities
