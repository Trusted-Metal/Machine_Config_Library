#pragma once
#include <string>
#include <vector>

namespace machine_config::capabilities {

struct CapabilityError {
  std::string code;
  std::string message;
  // Names every individual violation at once (e.g. every missing required
  // OPCUA field) rather than only the first one encountered. Empty for
  // errors with nothing more specific to list.
  std::vector<std::string> details;
};

inline CapabilityError capabilityError(std::string code, std::string message,
                                        std::vector<std::string> details = {}) {
  return CapabilityError{std::move(code), std::move(message), std::move(details)};
}

}  // namespace machine_config::capabilities
