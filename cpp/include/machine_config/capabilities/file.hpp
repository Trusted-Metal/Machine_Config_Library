#pragma once
// Version-agnostic dispatch: peek File_Version, then open the matching adapter.
// Version-specific facades live under capabilities/v1_0/, v1_1/, …
#include "machine_config/capabilities/errors.hpp"
#include "machine_config/capabilities/generated.hpp"
#include "machine_config/capabilities/result.hpp"
#include "machine_config/capabilities/v1_0/file.hpp"
#include "machine_config/reader.hpp"

#include <filesystem>
#include <memory>
#include <string>

namespace machine_config::capabilities {

inline Result<std::shared_ptr<IMachineConfigFile>> openMachineConfig(
    const std::filesystem::path& path) {
  try {
    std::string fv = ::machine_config::peekFileVersion(path);
    if (fv != "1.0") {
      return Result<std::shared_ptr<IMachineConfigFile>>::Err(
          "UnsupportedVersion",
          "No capability adapter registered for File_Version \"" + fv + "\"");
    }
  } catch (const std::exception& e) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err("IoError", e.what());
  }
  auto r = MachineConfigFileV1_0::open(path);
  if (!r.ok()) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err(
        r.errorCode(), r.errorMessage(), r.errorDetails());
  }
  return Result<std::shared_ptr<IMachineConfigFile>>::Ok(
      std::static_pointer_cast<IMachineConfigFile>(r.value()));
}

inline Result<std::shared_ptr<MachineConfigFileV1_0>> createMachineConfig(
    const std::string& version) {
  std::string fv = version.empty() ? "1.0" : version;
  if (fv != "1.0") {
    return Result<std::shared_ptr<MachineConfigFileV1_0>>::Err(
        "UnsupportedVersion", "create() unsupported for File_Version \"" + fv + "\"");
  }
  return MachineConfigFileV1_0::create(fv);
}

}  // namespace machine_config::capabilities
