#pragma once
// Version-agnostic dispatch: peek File_Version, then open the matching adapter.
// Version-specific facades live under capabilities/v1_0/, v1_1/, …
#include "machine_config/capabilities/errors.hpp"
#include "machine_config/capabilities/generated.hpp"
#include "machine_config/capabilities/result.hpp"
#include "machine_config/capabilities/v1_0/file.hpp"
#include "machine_config/reader.hpp"

#include <filesystem>
#include <functional>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

namespace machine_config::capabilities {

// Version -> constructor for that version's facade, already type-erased to
// the common IMachineConfigFile interface. A real registry
// (DISPATCH_REGISTRY_PLAN.md): adding a version means adding an entry here,
// never editing openMachineConfig itself.
using OpenRegistry =
    std::unordered_map<std::string,
                        std::function<Result<std::shared_ptr<IMachineConfigFile>>(
                            const std::filesystem::path&)>>;

inline const OpenRegistry& productionOpenRegistry() {
  static const OpenRegistry registry = {
      {"1.0",
       [](const std::filesystem::path& path) -> Result<std::shared_ptr<IMachineConfigFile>> {
         auto r = MachineConfigFileV1_0::open(path);
         if (!r.ok()) {
           return Result<std::shared_ptr<IMachineConfigFile>>::Err(
               r.errorCode(), r.errorMessage(), r.errorDetails());
         }
         return Result<std::shared_ptr<IMachineConfigFile>>::Ok(
             std::static_pointer_cast<IMachineConfigFile>(r.value()));
       }},
  };
  return registry;
}

// Registry-parameterized so tests can inject a fake entry without touching
// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
inline Result<std::shared_ptr<IMachineConfigFile>> openMachineConfigWithRegistry(
    const std::filesystem::path& path, const OpenRegistry& registry) {
  std::string fv;
  try {
    fv = ::machine_config::peekFileVersion(path);
  } catch (const std::exception& e) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err("IoError", e.what());
  }
  auto it = registry.find(fv);
  if (it == registry.end()) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err(
        "UnsupportedVersion",
        "No capability adapter registered for File_Version \"" + fv + "\"");
  }
  return it->second(path);
}

inline Result<std::shared_ptr<IMachineConfigFile>> openMachineConfig(
    const std::filesystem::path& path) {
  return openMachineConfigWithRegistry(path, productionOpenRegistry());
}

// Same shape as OpenRegistry/openMachineConfigWithRegistry, for create().
using CreateRegistry =
    std::unordered_map<std::string,
                        std::function<Result<std::shared_ptr<IMachineConfigFile>>(
                            const std::string&)>>;

inline const CreateRegistry& productionCreateRegistry() {
  static const CreateRegistry registry = {
      {"1.0",
       [](const std::string& version) -> Result<std::shared_ptr<IMachineConfigFile>> {
         auto r = MachineConfigFileV1_0::create(version);
         if (!r.ok()) {
           return Result<std::shared_ptr<IMachineConfigFile>>::Err(
               r.errorCode(), r.errorMessage(), r.errorDetails());
         }
         return Result<std::shared_ptr<IMachineConfigFile>>::Ok(
             std::static_pointer_cast<IMachineConfigFile>(r.value()));
       }},
  };
  return registry;
}

inline Result<std::shared_ptr<IMachineConfigFile>> createMachineConfigWithRegistry(
    const std::string& version, const CreateRegistry& registry) {
  std::string fv = version.empty() ? "1.0" : version;
  auto it = registry.find(fv);
  if (it == registry.end()) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err(
        "UnsupportedVersion", "create() unsupported for File_Version \"" + fv + "\"");
  }
  return it->second(fv);
}

inline Result<std::shared_ptr<IMachineConfigFile>> createMachineConfig(
    const std::string& version) {
  return createMachineConfigWithRegistry(version, productionCreateRegistry());
}

// Registered File_Version strings this dispatcher understands — mirrors
// Python's supported_file_versions(), Node's supportedFileVersions(),
// Rust's supported_file_versions(), and Go's SupportedFileVersions(). Pure
// introspection: callers never need this before open()/create(), which
// already dispatch/validate File_Version internally (see openMachineConfig
// above); this exists for callers who want to ask "what do you support?"
// ahead of time (e.g. to validate a batch of files or show it in a UI/log).
// Derived from the registry's own keys, not a separately-maintained literal.
inline std::vector<std::string> supportedFileVersions() {
  std::vector<std::string> versions;
  versions.reserve(productionOpenRegistry().size());
  for (const auto& [version, _] : productionOpenRegistry()) versions.push_back(version);
  return versions;
}

}  // namespace machine_config::capabilities
