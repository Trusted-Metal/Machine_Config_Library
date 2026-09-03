#pragma once
// File_Version 1.1 on-disk HDF5 layout.
//
// Deliberately independent of any other version's own layout header — see
// the isolation rule enforced by
// cpp/tests/test_version_adapter_isolation.cpp: nothing in this directory
// may reference another File_Version's adapter code, even for logic that
// happens to be identical between versions today, so this version can
// change or be removed later without anyone needing to check a sibling
// version first.

#include <iomanip>
#include <sstream>
#include <string>

namespace machine_config::capabilities::v1_1 {

inline constexpr const char* FILE_VERSION = "1.1";

inline constexpr const char* ROOT_MACHINE = "Machine";
inline constexpr const char* ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains";
inline constexpr const char* ROOT_EXTENSIONS = "Extensions";
inline constexpr const char* GROUP_CLEARBOX = "Extensions/ClearBox";
inline constexpr const char* ROOT_OPCUA = "Extensions/TM_OPCUA";
inline constexpr const char* OPCUA_CLIENT = "Extensions/TM_OPCUA/Client";
inline constexpr const char* OPCUA_PIPE = "Extensions/TM_OPCUA/Pipe";
inline constexpr const char* OPCUA_TRIGGERS = "Extensions/TM_OPCUA/Triggers";

inline constexpr const char* TRAIN_ID_PREFIX = "Optical_Train_";
inline constexpr const char* GROUP_SCANNER = "Scanner";
inline constexpr const char* GROUP_LIGHT_SOURCE = "Light_Source";
inline constexpr const char* GROUP_COLLIMATOR = "Collimator";
inline constexpr const char* GROUP_SCANNER_CARD = "Scanner_Card";
inline constexpr const char* GROUP_POWER_CHARACTERIZATION = "Power_Characterization";
inline constexpr const char* DS_DERIVATION_EQUATION_CONSTANTS = "Derivation_Equation_Constants";
inline constexpr const char* DS_CHARACTERIZATION_POINTS = "Characterization_Points";
inline constexpr const char* DS_CORRECTION_DATA = "Correction_Data";
inline constexpr const char* DS_INVERSE_CORRECTION_DATA = "Inverse_Correction_Data";
inline constexpr const char* DS_SCAN_FIELD_CORRECTION_FILE = "scan_field_correction_file";

inline std::string trainId(std::size_t index) {
  std::ostringstream os;
  os << TRAIN_ID_PREFIX << std::setw(2) << std::setfill('0') << (index + 1);
  return os.str();
}

inline std::string trainPathById(const std::string& tid) {
  return std::string(ROOT_OPTICAL_TRAINS) + "/" + tid;
}

inline std::string scannerPathById(const std::string& tid) {
  return trainPathById(tid) + "/" + GROUP_SCANNER;
}

inline std::string lightSourcePathById(const std::string& tid) {
  return trainPathById(tid) + "/" + GROUP_LIGHT_SOURCE;
}

inline std::string lightSourcePowerCharacterizationPathById(const std::string& tid) {
  return lightSourcePathById(tid) + "/" + GROUP_POWER_CHARACTERIZATION;
}

inline std::string collimatorPathById(const std::string& tid) {
  return trainPathById(tid) + "/" + GROUP_COLLIMATOR;
}

inline std::string scannerCardPathById(const std::string& tid) {
  return trainPathById(tid) + "/" + GROUP_SCANNER_CARD;
}

inline std::string clearboxPathById(const std::string& tid) {
  return std::string(GROUP_CLEARBOX) + "/" + tid;
}

inline std::string clearboxPowerCharacterizationPathById(const std::string& tid) {
  return clearboxPathById(tid) + "/" + GROUP_POWER_CHARACTERIZATION;
}

inline std::string correctionDataPathById(const std::string& tid) {
  return clearboxPathById(tid) + "/" + DS_CORRECTION_DATA;
}

inline std::string inverseCorrectionDataPathById(const std::string& tid) {
  return clearboxPathById(tid) + "/" + DS_INVERSE_CORRECTION_DATA;
}

inline std::string scanFieldCorrectionFilePathById(const std::string& tid) {
  return trainPathById(tid) + "/" + DS_SCAN_FIELD_CORRECTION_FILE;
}

}  // namespace machine_config::capabilities::v1_1
