// AUTO-GENERATED from schema/capabilities — DO NOT EDIT
// python tools/generate_capabilities.py

#pragma once
#include "machine_config/capabilities/result.hpp"
#include <string>

namespace machine_config::capabilities {

enum class SetMode { Merge, Replace };

class IMachineConfigFile {
public:
  virtual ~IMachineConfigFile() = default;
  virtual std::string fileVersion() const = 0;
  virtual std::size_t opticalTrainCount() const = 0;
  virtual Result<void> save(const std::string* path = nullptr) = 0;
  virtual void close() = 0;
};

}  // namespace machine_config::capabilities
