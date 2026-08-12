#pragma once
#include "machine_config/capabilities/errors.hpp"
#include "machine_config/capabilities/generated.hpp"
#include "machine_config/capabilities/result.hpp"
#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#include <filesystem>
#include <memory>
#include <string>
#include <utility>

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

class MachineConfigFileV10 : public IMachineConfigFile {
 public:
  static Result<std::shared_ptr<MachineConfigFileV10>> open(const std::filesystem::path& path) {
    try {
      MachineConfigReader reader(path.string());
      auto config = reader.parseWithBinary();
      std::string fv = config.meta.file_version;
      while (!fv.empty() && (fv.back() == ' ' || fv.back() == '\n' || fv.back() == '\r'))
        fv.pop_back();
      if (fv.empty()) fv = "1.0";
      if (fv != "1.0") {
        return Result<std::shared_ptr<MachineConfigFileV10>>::Err(
            "UnsupportedVersion",
            "No adapter for File_Version \"" + fv + "\" (v1.0 facade)");
      }
      return Result<std::shared_ptr<MachineConfigFileV10>>::Ok(
          std::shared_ptr<MachineConfigFileV10>(
              new MachineConfigFileV10(std::move(config), path, fv)));
    } catch (const std::exception& e) {
      return Result<std::shared_ptr<MachineConfigFileV10>>::Err("IoError", e.what());
    }
  }

  static Result<std::shared_ptr<MachineConfigFileV10>> create(const std::string& version) {
    std::string fv = version.empty() ? "1.0" : version;
    if (fv != "1.0") {
      return Result<std::shared_ptr<MachineConfigFileV10>>::Err(
          "UnsupportedVersion", "create() unsupported for File_Version \"" + fv + "\"");
    }
    try {
      MockConfigBuilder b;
      b.laser_count = 1;
      MachineConfig config = b.build();
      config.meta.file_version = "1.0";
      return Result<std::shared_ptr<MachineConfigFileV10>>::Ok(
          std::shared_ptr<MachineConfigFileV10>(
              new MachineConfigFileV10(std::move(config), {}, "1.0")));
    } catch (const std::exception& e) {
      return Result<std::shared_ptr<MachineConfigFileV10>>::Err("IoError", e.what());
    }
  }

  std::string fileVersion() const override { return file_version_; }
  std::size_t opticalTrainCount() const override {
    assertOpen();
    return config_.optical_trains.size();
  }

  MachineConfigMeta getMeta() const {
    assertOpen();
    return config_.meta;
  }
  Result<void> setMeta(const MachineConfigMeta& model, SetMode mode = SetMode::Merge) {
    assertOpen();
    config_.meta = applySetModeT(config_.meta, model, mode);
    return Result<void>::Ok();
  }

  Machine getMachine() const {
    assertOpen();
    return config_.machine;
  }
  Result<void> setMachine(const Machine& model, SetMode mode = SetMode::Merge) {
    assertOpen();
    config_.machine = applySetModeT(config_.machine, model, mode);
    return Result<void>::Ok();
  }

  Result<OpticalTrain> getTrain(std::size_t index) const {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<OpticalTrain>::Err(
          "InvalidIndex",
          "optical train index out of range");
    }
    return Result<OpticalTrain>::Ok(config_.optical_trains[index]);
  }

  Result<Scanner> getScanner(std::size_t index) const {
    auto t = getTrain(index);
    if (!t.ok()) return Result<Scanner>::Err(t.errorCode(), t.errorMessage());
    return Result<Scanner>::Ok(t.value().scanner);
  }

  Result<void> setScanner(std::size_t index, const Scanner& model, SetMode mode = SetMode::Merge) {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].scanner =
        applySetModeT(config_.optical_trains[index].scanner, model, mode);
    return Result<void>::Ok();
  }

  Result<LightSource> getLightSource(std::size_t index) const {
    auto t = getTrain(index);
    if (!t.ok()) return Result<LightSource>::Err(t.errorCode(), t.errorMessage());
    return Result<LightSource>::Ok(t.value().light_source);
  }

  Result<void> setLightSource(std::size_t index, const LightSource& model,
                              SetMode mode = SetMode::Merge) {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].light_source =
        applySetModeT(config_.optical_trains[index].light_source, model, mode);
    return Result<void>::Ok();
  }

  Result<Collimator> getCollimator(std::size_t index) const {
    auto t = getTrain(index);
    if (!t.ok()) return Result<Collimator>::Err(t.errorCode(), t.errorMessage());
    return Result<Collimator>::Ok(t.value().collimator);
  }

  Result<void> setCollimator(std::size_t index, const Collimator& model,
                             SetMode mode = SetMode::Merge) {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].collimator =
        applySetModeT(config_.optical_trains[index].collimator, model, mode);
    return Result<void>::Ok();
  }

  Result<ScannerCard> getScannerCard(std::size_t index) const {
    auto t = getTrain(index);
    if (!t.ok()) return Result<ScannerCard>::Err(t.errorCode(), t.errorMessage());
    return Result<ScannerCard>::Ok(t.value().scanner_card);
  }

  Result<void> setScannerCard(std::size_t index, const ScannerCard& model,
                              SetMode mode = SetMode::Merge) {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].scanner_card =
        applySetModeT(config_.optical_trains[index].scanner_card, model, mode);
    return Result<void>::Ok();
  }

  bool hasOptionalComponents(std::size_t index) const {
    assertOpen();
    if (index >= config_.optical_trains.size()) return false;
    return config_.optical_trains[index].optional_components.clearbox.has_value();
  }

  Result<ClearBox> getClearbox(std::size_t index) const {
    auto t = getTrain(index);
    if (!t.ok()) return Result<ClearBox>::Err(t.errorCode(), t.errorMessage());
    if (!t.value().optional_components.clearbox.has_value()) {
      return Result<ClearBox>::Err("NotPresent", "clearbox is not present");
    }
    return Result<ClearBox>::Ok(*t.value().optional_components.clearbox);
  }

  Result<void> setClearbox(std::size_t index, const ClearBox& model,
                           SetMode mode = SetMode::Merge) {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    auto& slot = config_.optical_trains[index].optional_components.clearbox;
    if (!slot.has_value()) {
      return Result<void>::Err("NotPresent", "clearbox is not present");
    }
    slot = applySetModeT(*slot, model, mode);
    return Result<void>::Ok();
  }

  Result<OpcuaConfig> getOpcua() const {
    assertOpen();
    if (!config_.opcua.has_value()) {
      return Result<OpcuaConfig>::Err("NotPresent", "OPCUA group is not present");
    }
    return Result<OpcuaConfig>::Ok(*config_.opcua);
  }

  Result<void> setOpcua(const OpcuaConfig& model, SetMode mode = SetMode::Merge) {
    assertOpen();
    if (!config_.opcua.has_value()) {
      return Result<void>::Err("NotPresent", "OPCUA group is not present");
    }
    config_.opcua = applySetModeT(*config_.opcua, model, mode);
    return Result<void>::Ok();
  }

  Result<void> save(const std::string* path = nullptr) override {
    assertOpen();
    try {
      std::filesystem::path out = path ? std::filesystem::path(*path) : path_;
      if (out.empty()) {
        return Result<void>::Err("ValidationError",
                                 "save() requires a path for create()-d files");
      }
      MachineConfigWriter{config_}.write(out.string());
      path_ = out;
      return Result<void>::Ok();
    } catch (const std::exception& e) {
      return Result<void>::Err("IoError", e.what());
    }
  }

  void close() override { closed_ = true; }

 private:
  MachineConfigFileV10(MachineConfig config, std::filesystem::path path, std::string fv)
      : config_(std::move(config)), path_(std::move(path)), file_version_(std::move(fv)) {}

  void assertOpen() const {
    if (closed_) throw std::runtime_error("MachineConfigFile session is closed");
  }

  MachineConfig config_;
  std::filesystem::path path_;
  std::string file_version_;
  bool closed_ = false;
};

inline Result<std::shared_ptr<IMachineConfigFile>> openMachineConfig(
    const std::filesystem::path& path) {
  auto r = MachineConfigFileV10::open(path);
  if (!r.ok()) {
    return Result<std::shared_ptr<IMachineConfigFile>>::Err(r.errorCode(), r.errorMessage());
  }
  return Result<std::shared_ptr<IMachineConfigFile>>::Ok(
      std::static_pointer_cast<IMachineConfigFile>(r.value()));
}

inline Result<std::shared_ptr<MachineConfigFileV10>> createMachineConfig(
    const std::string& version) {
  return MachineConfigFileV10::create(version);
}

}  // namespace machine_config::capabilities
