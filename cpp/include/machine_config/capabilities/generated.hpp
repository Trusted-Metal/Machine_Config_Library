// AUTO-GENERATED from schema/capabilities — DO NOT EDIT
// python tools/generate_capabilities.py

#pragma once
#include "machine_config/capabilities/result.hpp"
#include "machine_config/models.hpp"
#include <cstddef>
#include <string>

namespace machine_config::capabilities {

enum class SetMode { Merge, Replace };

class IMachineConfigFile {
public:
  virtual ~IMachineConfigFile() = default;
  virtual std::string fileVersion() const = 0;
  virtual std::size_t opticalTrainCount() const = 0;

  virtual MachineConfigMeta getMeta() const = 0;
  virtual Result<void> setMeta(const MachineConfigMeta& model, SetMode mode = SetMode::Merge) = 0;

  virtual Machine getMachine() const = 0;
  virtual Result<void> setMachine(const Machine& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<OpticalTrain> getTrain(std::size_t index) const = 0;

  virtual Result<Scanner> getScanner(std::size_t index) const = 0;
  virtual Result<void> setScanner(std::size_t index, const Scanner& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<LightSource> getLightSource(std::size_t index) const = 0;
  virtual Result<void> setLightSource(std::size_t index, const LightSource& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<Collimator> getCollimator(std::size_t index) const = 0;
  virtual Result<void> setCollimator(std::size_t index, const Collimator& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<ScannerCard> getScannerCard(std::size_t index) const = 0;
  virtual Result<void> setScannerCard(std::size_t index, const ScannerCard& model, SetMode mode = SetMode::Merge) = 0;

  virtual bool hasOptionalComponents(std::size_t index) const = 0;
  virtual Result<ClearBox> getClearbox(std::size_t index) const = 0;
  virtual Result<void> setClearbox(std::size_t index, const ClearBox& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<CorrectionData> getCorrectionData(std::size_t index) const = 0;
  virtual Result<CorrectionData> getInverseCorrectionData(std::size_t index) const = 0;

  virtual Result<OpcuaConfig> getOpcua() const = 0;
  virtual Result<void> setOpcua(const OpcuaConfig& model, SetMode mode = SetMode::Merge) = 0;

  virtual Result<void> save(const std::string* path = nullptr) = 0;
  virtual void close() = 0;
};

}  // namespace machine_config::capabilities
