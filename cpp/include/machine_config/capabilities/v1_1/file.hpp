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
#include "machine_config/hash.hpp"
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

  // Recomputes the configuration hash from the session's *current* content
  // and compares it to the session's current meta.configuration_hash --
  // fresh, every call. Deliberately live, not cached at open() time: this
  // facade is a mutable session (setMeta/setScanner/etc. all mutate
  // config_ in place), so a value computed once at open() would silently
  // go stale the instant any set* call ran. See
  // CAPABILITIES_HASH_INTEGRATION_PLAN.md's live-method design -- in
  // particular, this reads false for the entire span between an edit and
  // the next save(), which is expected (an unsaved edit hasn't been hashed
  // yet), not a sign of corruption. Never throws: a mismatch is
  // informational, matching CONFIGURATION_HASH_PLAN.md's non-fatal design
  // throughout.
  bool isValid() const override {
    return computeConfigurationHash(config_) == config_.meta.configuration_hash;
  }

  Result<void> save(const std::string* path = nullptr) override {
    assertOpen();
    try {
      std::filesystem::path out = path ? std::filesystem::path(*path) : path_;
      if (out.empty()) {
        return Result<void>::Err("ValidationError",
                                 "save() requires a path for create()-d files");
      }
      // Compute-and-stamp on the session's own config first (not just
      // whatever MachineConfigWriter computes internally on its own copy)
      // so isValid() reads true immediately after save(), without
      // requiring a fresh open() -- see
      // CAPABILITIES_HASH_INTEGRATION_PLAN.md's live-method design.
      config_.meta.configuration_hash = computeConfigurationHash(config_);
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
