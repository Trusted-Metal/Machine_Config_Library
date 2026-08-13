#pragma once
// File_Version 1.0 on-disk HDF5 layout.

#include <iomanip>
#include <sstream>
#include <string>

namespace machine_config::capabilities::v1_0 {

inline constexpr const char* FILE_VERSION = "1.0";
inline constexpr const char* ROOT_MACHINE = "Machine";
inline constexpr const char* ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains";
inline constexpr const char* ROOT_OPCUA = "OPCUA";
inline constexpr const char* TRAIN_ID_PREFIX = "Optical_Train_";
inline constexpr const char* GROUP_OPTIONAL_COMPONENTS = "Optional_Components";
inline constexpr const char* GROUP_CLEARBOX = "ClearBox";
inline constexpr const char* DS_CORRECTION_DATA = "Correction_Data";
inline constexpr const char* DS_INVERSE_CORRECTION_DATA = "Inverse_Correction_Data";
inline constexpr const char* DS_SCAN_FIELD_CORRECTION_FILE = "scan_field_correction_file";

inline std::string trainId(std::size_t index) {
  std::ostringstream os;
  os << TRAIN_ID_PREFIX << std::setw(2) << std::setfill('0') << (index + 1);
  return os.str();
}

inline std::string trainPath(std::size_t index) {
  return std::string(ROOT_OPTICAL_TRAINS) + "/" + trainId(index);
}

inline std::string trainPathById(const std::string& tid) {
  return std::string(ROOT_OPTICAL_TRAINS) + "/" + tid;
}

inline std::string clearboxPath(std::size_t index) {
  return trainPath(index) + "/" + GROUP_OPTIONAL_COMPONENTS + "/" + GROUP_CLEARBOX;
}

inline std::string clearboxPathById(const std::string& tid) {
  return trainPathById(tid) + "/" + GROUP_OPTIONAL_COMPONENTS + "/" + GROUP_CLEARBOX;
}

}  // namespace machine_config::capabilities::v1_0
