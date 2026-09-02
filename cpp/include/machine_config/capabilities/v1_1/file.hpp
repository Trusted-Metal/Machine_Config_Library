#pragma once
// File_Version 1.1 stable-model facade (session + get/set). Mirrors the
// structure of the facade for this project's other production File_Version,
// but is defined fully independently here — see the isolation rule
// enforced by cpp/tests/test_version_adapter_isolation.cpp: nothing in this
// directory may reference another File_Version's adapter code or class
// names, even where the facade logic is identical in shape.

#include "machine_config/builder.hpp"
#include "machine_config/capabilities/generated.hpp"
#include "machine_config/capabilities/merge.hpp"
#include "machine_config/capabilities/result.hpp"
#include "machine_config/capabilities/v1_1/hdf5.hpp"
#include "machine_config/writer.hpp"

#include <filesystem>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace machine_config::capabilities {

class MachineConfigFileV1_1 : public IMachineConfigFile {
 public:
  static Result<std::shared_ptr<MachineConfigFileV1_1>> open(const std::filesystem::path& path) {
    try {
      v1_1::Hdf5AdapterV1_1 reader(path.string());
      auto config = reader.parseWithBinary();
      std::string fv = config.meta.file_version;
      while (!fv.empty() && (fv.back() == ' ' || fv.back() == '\n' || fv.back() == '\r'))
        fv.pop_back();
      if (fv.empty()) fv = "1.1";
      if (fv != "1.1") {
        return Result<std::shared_ptr<MachineConfigFileV1_1>>::Err(
            "UnsupportedVersion",
            "No adapter for File_Version \"" + fv + "\" (v1.1 facade)");
      }
      return Result<std::shared_ptr<MachineConfigFileV1_1>>::Ok(
          std::shared_ptr<MachineConfigFileV1_1>(
              new MachineConfigFileV1_1(std::move(config), path, fv)));
    } catch (const std::exception& e) {
      return Result<std::shared_ptr<MachineConfigFileV1_1>>::Err("IoError", e.what());
    }
  }

  // Builds a plain previous-version-shaped mock config — MockConfigBuilder
  // already builds power_characterization natively, so nothing further needs
  // deriving here.
  static Result<std::shared_ptr<MachineConfigFileV1_1>> create(const std::string& version) {
    std::string fv = version.empty() ? "1.1" : version;
    if (fv != "1.1") {
      return Result<std::shared_ptr<MachineConfigFileV1_1>>::Err(
          "UnsupportedVersion", "create() unsupported for File_Version \"" + fv + "\"");
    }
    try {
      MockConfigBuilder b;
      b.laser_count = 1;
      MachineConfig config = b.build();
      config.meta.file_version = "1.1";
      return Result<std::shared_ptr<MachineConfigFileV1_1>>::Ok(
          std::shared_ptr<MachineConfigFileV1_1>(
              new MachineConfigFileV1_1(std::move(config), {}, "1.1")));
    } catch (const std::exception& e) {
      return Result<std::shared_ptr<MachineConfigFileV1_1>>::Err("IoError", e.what());
    }
  }

  std::string fileVersion() const override { return file_version_; }
  std::size_t opticalTrainCount() const override {
    assertOpen();
    return config_.optical_trains.size();
  }

  MachineConfigMeta getMeta() const override {
    assertOpen();
    return config_.meta;
  }
  Result<void> setMeta(const MachineConfigMeta& model, SetMode mode = SetMode::Merge) override {
    assertOpen();
    config_.meta = applySetModeT(config_.meta, model, mode);
    return Result<void>::Ok();
  }

  Machine getMachine() const override {
    assertOpen();
    return config_.machine;
  }
  Result<void> setMachine(const Machine& model, SetMode mode = SetMode::Merge) override {
    assertOpen();
    config_.machine = applySetModeT(config_.machine, model, mode);
    return Result<void>::Ok();
  }

  Result<OpticalTrain> getTrain(std::size_t index) const override {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<OpticalTrain>::Err(
          "InvalidIndex",
          "optical train index out of range");
    }
    return Result<OpticalTrain>::Ok(config_.optical_trains[index]);
  }

  Result<Scanner> getScanner(std::size_t index) const override {
    auto t = getTrain(index);
    if (!t.ok()) return Result<Scanner>::Err(t.errorCode(), t.errorMessage());
    return Result<Scanner>::Ok(t.value().scanner);
  }

  Result<void> setScanner(std::size_t index, const Scanner& model, SetMode mode = SetMode::Merge) override {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].scanner =
        applySetModeT(config_.optical_trains[index].scanner, model, mode);
    return Result<void>::Ok();
  }

  Result<LightSource> getLightSource(std::size_t index) const override {
    auto t = getTrain(index);
    if (!t.ok()) return Result<LightSource>::Err(t.errorCode(), t.errorMessage());
    return Result<LightSource>::Ok(t.value().light_source);
  }

  Result<void> setLightSource(std::size_t index, const LightSource& model,
                              SetMode mode = SetMode::Merge) override {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].light_source =
        applySetModeT(config_.optical_trains[index].light_source, model, mode);
    return Result<void>::Ok();
  }

  Result<Collimator> getCollimator(std::size_t index) const override {
    auto t = getTrain(index);
    if (!t.ok()) return Result<Collimator>::Err(t.errorCode(), t.errorMessage());
    return Result<Collimator>::Ok(t.value().collimator);
  }

  Result<void> setCollimator(std::size_t index, const Collimator& model,
                             SetMode mode = SetMode::Merge) override {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].collimator =
        applySetModeT(config_.optical_trains[index].collimator, model, mode);
    return Result<void>::Ok();
  }

  Result<ScannerCard> getScannerCard(std::size_t index) const override {
    auto t = getTrain(index);
    if (!t.ok()) return Result<ScannerCard>::Err(t.errorCode(), t.errorMessage());
    return Result<ScannerCard>::Ok(t.value().scanner_card);
  }

  Result<void> setScannerCard(std::size_t index, const ScannerCard& model,
                              SetMode mode = SetMode::Merge) override {
    assertOpen();
    if (index >= config_.optical_trains.size()) {
      return Result<void>::Err("InvalidIndex", "optical train index out of range");
    }
    config_.optical_trains[index].scanner_card =
        applySetModeT(config_.optical_trains[index].scanner_card, model, mode);
    return Result<void>::Ok();
  }

  bool hasOptionalComponents(std::size_t index) const override {
    assertOpen();
    if (index >= config_.optical_trains.size()) return false;
    return config_.optical_trains[index].optional_components.clearbox.has_value();
  }

  Result<ClearBox> getClearbox(std::size_t index) const override {
    auto t = getTrain(index);
    if (!t.ok()) return Result<ClearBox>::Err(t.errorCode(), t.errorMessage());
    if (!t.value().optional_components.clearbox.has_value()) {
      return Result<ClearBox>::Err("NotPresent", "clearbox is not present");
    }
    return Result<ClearBox>::Ok(*t.value().optional_components.clearbox);
  }

  Result<void> setClearbox(std::size_t index, const ClearBox& model,
                           SetMode mode = SetMode::Merge) override {
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

  // Converts the already-loaded in-memory ClearBox model rather than
  // re-reading the file, so this works for both open()- and create()-based
  // instances alike.
  Result<CorrectionData> getCorrectionData(std::size_t index) const override {
    auto cb = getClearbox(index);
    if (!cb.ok()) return Result<CorrectionData>::Err(cb.errorCode(), cb.errorMessage());
    CorrectionData cd;
    cd.data = detail::gridToFlat(cb.value().correction_data, cd.shape);
    return Result<CorrectionData>::Ok(std::move(cd));
  }

  Result<CorrectionData> getInverseCorrectionData(std::size_t index) const override {
    auto cb = getClearbox(index);
    if (!cb.ok()) return Result<CorrectionData>::Err(cb.errorCode(), cb.errorMessage());
    CorrectionData cd;
    cd.data = detail::gridToFlat(cb.value().inverse_correction_data, cd.shape);
    return Result<CorrectionData>::Ok(std::move(cd));
  }

  // Returns the OPCUA config, or Err("ValidationError", ...) if OPCUA is
  // present but missing one or more required fields. Collects every missing
  // field at once (in errorDetails()) rather than failing on the first one.
  // The low-level reader/writer stay fully permissive; this is the one
  // place "required" is enforced.
  Result<OpcuaConfig> getOpcua() const override {
    assertOpen();
    if (!config_.opcua.has_value()) {
      return Result<OpcuaConfig>::Err("NotPresent", "OPCUA group is not present");
    }
    const auto& opcua = *config_.opcua;

    std::vector<std::string> missing;
    if (!opcua.client.machine_profile.has_value()) missing.push_back("Machine_Profile");
    if (!opcua.client.root_node.has_value()) missing.push_back("Root_Node");
    if (!opcua.pipe.configure_client.has_value()) missing.push_back("Configure_Client");
    if (!opcua.pipe.pipe_name.has_value()) missing.push_back("Pipe_Name");
    if (!opcua.triggers_enabled.has_value()) missing.push_back("Triggers_Enabled");
    if (!opcua.trigger_stop_ceiling_layers.has_value()) missing.push_back("Trigger_Stop_Ceiling_Layers");
    for (const auto& [name, trigger] : opcua.triggers) {
      if (!trigger.event.has_value()) missing.push_back(name + ".Event");
    }

    if (!missing.empty()) {
      std::string msg = "OPCUA is present but missing required field(s): ";
      for (std::size_t i = 0; i < missing.size(); ++i) {
        if (i) msg += ", ";
        msg += missing[i];
      }
      return Result<OpcuaConfig>::Err("ValidationError", msg, missing);
    }

    return Result<OpcuaConfig>::Ok(opcua);
  }

  Result<void> setOpcua(const OpcuaConfig& model, SetMode mode = SetMode::Merge) override {
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
  MachineConfigFileV1_1(MachineConfig config, std::filesystem::path path, std::string fv)
      : config_(std::move(config)), path_(std::move(path)), file_version_(std::move(fv)) {}

  void assertOpen() const {
    if (closed_) throw std::runtime_error("MachineConfigFile session is closed");
  }

  MachineConfig config_;
  std::filesystem::path path_;
  std::string file_version_;
  bool closed_ = false;
};

}  // namespace machine_config::capabilities
