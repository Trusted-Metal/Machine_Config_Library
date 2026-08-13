#pragma once
// Machine Config Library — HDF5 reader (header-only, §4.8–§4.12).
// Each sub-step builds on the previous; stubs are filled in as each step completes.

#include "machine_config/models.hpp"
#include "machine_config/adapters/registry.hpp"

#include <highfive/H5File.hpp>

#include <algorithm>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <variant>

namespace machine_config {

static constexpr const char* CURRENT_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Attribute-reading helpers
// All HDF5 attribute access flows through these template functions so that
// they work uniformly for both HighFive::File and HighFive::Group.
// ---------------------------------------------------------------------------

// Decoded HDF5 attribute value; type matches how it is stored in the file.
using RawAttr = std::variant<std::string, int64_t, double>;

// Read one attribute by name; returns nullopt when the key does not exist.
// Dispatches on the HDF5 storage type (string / integer / float).
template <typename Loc>
inline std::optional<RawAttr> readRaw(const Loc& loc, const std::string& key) {
    if (!loc.hasAttribute(key)) return std::nullopt;
    auto attr = loc.getAttribute(key);
    switch (attr.getDataType().getClass()) {
        case HighFive::DataTypeClass::Float:
            return RawAttr{attr.template read<double>()};
        case HighFive::DataTypeClass::Integer:
            return RawAttr{attr.template read<int64_t>()};
        default: { // String
            // HighFive's read<string> mishandles VarLen strings whose strpad is
            // H5T_STR_SPACEPAD — it sets string_length=SIZE_MAX and std::string::assign
            // throws length_error.  These HDF5 files use strpad=2 (SpacePadded) for all
            // VarLen attrs.  Also, HDF5 1.14 cannot convert between SpacePadded and
            // NullTerminated VarLen types.  Fix: read VarLen attrs using the file's own
            // type directly; HDF5 always allocates a null-terminated heap pointer for VarLen
            // data regardless of the strpad metadata, so the result is always safe to use as
            // a C string.
            auto dtype = attr.getDataType();
            hid_t dtype_id = dtype.getId();
            std::string result;
            if (H5Tis_variable_str(dtype_id) > 0) {
                char* ptr = nullptr;
                if (H5Aread(attr.getId(), dtype_id, &ptr) >= 0 && ptr) {
                    result = ptr;
                    H5free_memory(ptr);
                }
            } else {
                // Fixed-length: read raw bytes, stop at first null byte.
                size_t sz = H5Tget_size(dtype_id);
                std::vector<char> buf(sz + 1, '\0');
                H5Aread(attr.getId(), dtype_id, buf.data());
                result = buf.data();
            }
            return RawAttr{std::move(result)};
        }
    }
}

// Converts a RawAttr to string; int/float fall back to decimal representation.
static inline std::string rawToString(const RawAttr& raw) {
    return std::visit([](auto&& v) -> std::string {
        using T = std::decay_t<decltype(v)>;
        if constexpr (std::is_same_v<T, std::string>) return v;
        else return std::to_string(v);
    }, raw);
}

// Mirrors Python's str(attrs.get(key, "")) — empty string when absent, no trimming.
template <typename Loc>
inline std::string readRequiredStr(const Loc& loc, const std::string& key) {
    auto raw = readRaw(loc, key);
    if (!raw) return {};
    return rawToString(*raw);
}

// Mirrors Python's _read_str — trims whitespace; returns nullopt when blank.
template <typename Loc>
inline std::optional<std::string> readStr(const Loc& loc, const std::string& key) {
    auto raw = readRaw(loc, key);
    if (!raw) return std::nullopt;
    std::string s = rawToString(*raw);
    auto first = s.find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return std::nullopt;
    auto last  = s.find_last_not_of(" \t\r\n");
    return s.substr(first, last - first + 1);
}

// Mirrors Python's _read_float — nullopt when absent or blank string.
template <typename Loc>
inline std::optional<double> readFloat(const Loc& loc, const std::string& key) {
    auto raw = readRaw(loc, key);
    if (!raw) return std::nullopt;
    return std::visit([&](auto&& v) -> std::optional<double> {
        using T = std::decay_t<decltype(v)>;
        if constexpr (std::is_same_v<T, double>) {
            return v;
        } else if constexpr (std::is_same_v<T, int64_t>) {
            return static_cast<double>(v);
        } else { // string — trim then parse
            std::string s = v;
            auto first = s.find_first_not_of(" \t\r\n");
            if (first == std::string::npos) return std::nullopt;
            auto last = s.find_last_not_of(" \t\r\n");
            s = s.substr(first, last - first + 1);
            if (s.empty()) return std::nullopt;
            try   { return std::stod(s); }
            catch (...) {
                throw std::runtime_error("attribute '" + key + "' has non-numeric string value: " + s);
            }
        }
    }, *raw);
}

// Mirrors Python's _read_int — nullopt when absent or blank string.
template <typename Loc>
inline std::optional<int64_t> readInt(const Loc& loc, const std::string& key) {
    auto raw = readRaw(loc, key);
    if (!raw) return std::nullopt;
    return std::visit([&](auto&& v) -> std::optional<int64_t> {
        using T = std::decay_t<decltype(v)>;
        if constexpr (std::is_same_v<T, int64_t>) {
            return v;
        } else if constexpr (std::is_same_v<T, double>) {
            return static_cast<int64_t>(v);
        } else { // string — trim then parse
            std::string s = v;
            auto first = s.find_first_not_of(" \t\r\n");
            if (first == std::string::npos) return std::nullopt;
            auto last = s.find_last_not_of(" \t\r\n");
            s = s.substr(first, last - first + 1);
            if (s.empty()) return std::nullopt;
            try   { return std::stoll(s); }
            catch (...) {
                throw std::runtime_error("attribute '" + key + "' has non-integer string value: " + s);
            }
        }
    }, *raw);
}

// Rule 8 for bool: 0 -> false, 1 -> true, other values -> error.
template <typename Loc>
inline std::optional<bool> readBoolFromInt(const Loc& loc, const std::string& key) {
    auto v = readInt(loc, key);
    if (!v) return std::nullopt;
    if (*v == 0) return false;
    if (*v == 1) return true;
    throw std::runtime_error(
        "attribute '" + key + "' has value " + std::to_string(*v) + "; expected 0 or 1 (Rule 8).");
}

// Rule 8: read a _unit attribute and assert it equals the schema-locked value.
template <typename Loc>
inline std::optional<std::string> readUnitLocked(
    const Loc& loc, const std::string& key, const std::string& expected)
{
    auto val = readStr(loc, key);
    if (!val) return std::nullopt;
    if (*val != expected)
        throw std::runtime_error(
            "Unit attribute '" + key + "' has value '" + *val +
            "'; expected '" + expected + "' (Rule 8).");
    return val;
}

// Collect all attributes not in `known` into an ExtraAttrs JSON object.
template <typename Loc>
inline ExtraAttrs collectExtra(const Loc& loc,
                               std::initializer_list<const char*> known)
{
    ExtraAttrs j = nlohmann::json::object();
    for (const auto& name : loc.listAttributeNames()) {
        if (std::any_of(known.begin(), known.end(),
                        [&](const char* k) { return name == k; }))
            continue;
        auto raw = readRaw(loc, name);
        if (!raw) continue;
        std::visit([&](auto&& v) { j[name] = v; }, *raw);
    }
    return j;
}

// ---------------------------------------------------------------------------
// MachineConfigReader
// ---------------------------------------------------------------------------

class MachineConfigReader {
public:
    explicit MachineConfigReader(std::filesystem::path path)
        : path_(std::move(path)) {}

    // Parse scalar / metadata fields only.  Correction grids and fc3 bytes
    // are left empty; use parseWithBinary() when those are needed (§4.11).
    MachineConfig parse() const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        return parseInner(f);
    }

    // Parse including full binary data — correction grids and raw fc3 bytes.
    // Required for copy-hdf5 (§4.15) and any caller needing complete fidelity.
    MachineConfig parseWithBinary() const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto cfg = parseInner(f);
        for (size_t i = 0; i < cfg.optical_trains.size(); ++i) {
            auto& ot  = cfg.optical_trains[i];
            auto  tid = trainIdAt(f, i);
            if (ot.optional_components.clearbox) {
                auto cd  = readCorrectionGrid(f,
                    "Machine/Optical_Trains/" + tid +
                    "/Optional_Components/ClearBox/Correction_Data");
                auto icd = readCorrectionGrid(f,
                    "Machine/Optical_Trains/" + tid +
                    "/Optional_Components/ClearBox/Inverse_Correction_Data");
                ot.optional_components.clearbox->correction_data         = correctionDataToGrid3D(cd);
                ot.optional_components.clearbox->inverse_correction_data = correctionDataToGrid3D(icd);
            }
            if (ot.scan_field_correction_file) {
                auto ds   = f.getDataSet("Machine/Optical_Trains/" + tid + "/scan_field_correction_file");
                auto dims = ds.getSpace().getDimensions();
                std::vector<uint8_t> bytes(dims[0]);
                H5Dread(ds.getId(), H5T_NATIVE_UINT8, H5S_ALL, H5S_ALL, H5P_DEFAULT, bytes.data());
                ot.scan_field_correction_file->raw_bytes = std::move(bytes);
            }
        }
        return cfg;
    }

    // Serialise to canonical JSON.  Correction grids excluded unless include_binary=true.
    std::string toJson(int indent = 2, bool /*include_binary*/ = false) const {
        nlohmann::json j = parse();
        return j.dump(indent);
    }

    // Returns all HDF5 attributes at hdf5_path as a JSON object.
    // Returns an empty object (never throws) when the path does not exist.
    nlohmann::json getRawGroup(const std::string& hdf5_path) const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        try {
            return collectExtra(f.getGroup(hdf5_path), {});
        } catch (...) {
            return nlohmann::json::object();
        }
    }

    // §4.11 — flat (d0×d1×d2) float64 correction grid for a 0-based train index.
    CorrectionData getCorrectionData(size_t train_index) const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        return readCorrectionGrid(f,
            "Machine/Optical_Trains/" + tid +
            "/Optional_Components/ClearBox/Correction_Data");
    }

    CorrectionData getInverseCorrectionData(size_t train_index) const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        return readCorrectionGrid(f,
            "Machine/Optical_Trains/" + tid +
            "/Optional_Components/ClearBox/Inverse_Correction_Data");
    }

    // §4.11 — raw .fc3 bytes embedded as a uint8 dataset.
    std::vector<uint8_t> getScanFieldCorrectionBytes(size_t train_index) const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        auto ds  = f.getDataSet("Machine/Optical_Trains/" + tid +
                                "/scan_field_correction_file");
        auto dims = ds.getSpace().getDimensions();
        std::vector<uint8_t> bytes(dims[0]);
        H5Dread(ds.getId(), H5T_NATIVE_UINT8, H5S_ALL, H5S_ALL, H5P_DEFAULT,
                bytes.data());
        return bytes;
    }

private:
    std::filesystem::path path_;

    // Snapshot all root attrs and each optical-train group's attrs into a JSON map.
    static nlohmann::json hdf5ToRaw(const HighFive::File& f) {
        nlohmann::json meta = nlohmann::json::object();
        for (const auto& name : f.listAttributeNames()) {
            auto raw = readRaw(f, name);
            if (!raw) continue;
            std::visit([&](auto&& v) { meta[name] = v; }, *raw);
        }

        nlohmann::json optical_trains = nlohmann::json::array();
        if (f.exist("Machine/Optical_Trains")) {
            auto trains_grp = f.getGroup("Machine/Optical_Trains");
            std::vector<std::string> ids;
            for (const auto& n : trains_grp.listObjectNames()) {
                if (n.size() >= 14 && n.substr(0, 14) == "Optical_Train_")
                    ids.push_back(n);
            }
            std::sort(ids.begin(), ids.end());
            for (const auto& tid : ids) {
                auto grp = trains_grp.getGroup(tid);
                nlohmann::json attrs = nlohmann::json::object();
                for (const auto& name : grp.listAttributeNames()) {
                    auto raw = readRaw(grp, name);
                    if (!raw) continue;
                    std::visit([&](auto&& v) { attrs[name] = v; }, *raw);
                }
                optical_trains.push_back(std::move(attrs));
            }
        }

        return nlohmann::json{{ "meta", meta }, { "optical_trains", optical_trains }};
    }

    // JSON-dict helpers for the adapter-output path.
    static std::optional<std::string> jsonStr(const nlohmann::json& j, const std::string& key) {
        if (!j.contains(key) || j.at(key).is_null()) return std::nullopt;
        std::string s = j.at(key).is_string() ? j.at(key).get<std::string>()
                                               : j.at(key).dump();
        auto first = s.find_first_not_of(" \t\r\n");
        if (first == std::string::npos) return std::nullopt;
        return s.substr(first, s.find_last_not_of(" \t\r\n") - first + 1);
    }
    static std::string jsonRequiredStr(const nlohmann::json& j, const std::string& key) {
        return jsonStr(j, key).value_or("");
    }
    static std::optional<double> jsonFloat(const nlohmann::json& j, const std::string& key) {
        if (!j.contains(key) || j.at(key).is_null()) return std::nullopt;
        if (j.at(key).is_number()) return j.at(key).get<double>();
        if (j.at(key).is_string()) {
            try { return std::stod(j.at(key).get<std::string>()); } catch (...) {}
        }
        return std::nullopt;
    }
    static std::optional<bool> jsonBoolFromInt(const nlohmann::json& j, const std::string& key) {
        if (!j.contains(key) || j.at(key).is_null()) return std::nullopt;
        int64_t v = j.at(key).is_number_integer() ? j.at(key).get<int64_t>()
                                                   : static_cast<int64_t>(j.at(key).get<double>());
        return v != 0;
    }

    void checkFileVersion(const HighFive::File& f) const {
        std::string ver = readRequiredStr(f, "File_Version");
        if (ver != CURRENT_VERSION)
            std::cerr << "Warning: File_Version is '" << ver
                      << "'; no adapter available. Output may be incomplete or incorrect.\n";
    }

    MachineConfig parseInner(const HighFive::File& f) const {
        std::string rawVersion = readRequiredStr(f, "File_Version");
        auto chain = adapters::AdapterRegistry::instance().get_chain(rawVersion, CURRENT_VERSION);

        // Warn for versions with no adapter and not current.
        if (chain.empty() && !rawVersion.empty() && rawVersion != CURRENT_VERSION)
            std::cerr << "Warning: File_Version is '" << rawVersion
                      << "'; no adapter available. Output may be incomplete or incorrect.\n";

        nlohmann::json adaptedMeta;
        std::vector<nlohmann::json> adaptedTrains;
        if (!chain.empty()) {
            auto raw = hdf5ToRaw(f);
            for (const auto& adapter : chain)
                raw = adapter->adapt(std::move(raw));
            raw["meta"]["File_Version"] = CURRENT_VERSION;
            adaptedMeta   = raw["meta"];
            if (raw["optical_trains"].is_array())
                for (auto& t : raw["optical_trains"])
                    adaptedTrains.push_back(std::move(t));
        }

        MachineConfig cfg;
        cfg.meta           = adaptedMeta.is_object() ? parseMetaFromJson(adaptedMeta) : parseMeta(f);
        cfg.machine        = parseMachine(f.getGroup("Machine"));
        cfg.optical_trains = parseTrains(f, adaptedTrains);
        cfg.opcua          = parseOpcua(f);
        return cfg;
    }

    MachineConfigMeta parseMeta(const HighFive::File& f) const {
        MachineConfigMeta m;
        m.machine_name       = readRequiredStr(f, "machine_name");
        m.manufacturer       = readRequiredStr(f, "manufacturer");
        m.model              = readRequiredStr(f, "model");
        m.serial_number      = readRequiredStr(f, "serial_number");
        m.file_version       = readRequiredStr(f, "File_Version");
        m.export_date        = readRequiredStr(f, "Export_Date");
        m.configuration_hash = readRequiredStr(f, "Configuration_Hash");
        m.extra              = collectExtra(f, {
            "machine_name", "manufacturer", "model", "serial_number",
            "File_Version", "Export_Date", "Configuration_Hash"
        });
        return m;
    }

    MachineConfigMeta parseMetaFromJson(const nlohmann::json& j) const {
        MachineConfigMeta m;
        m.machine_name       = jsonRequiredStr(j, "machine_name");
        m.manufacturer       = jsonRequiredStr(j, "manufacturer");
        m.model              = jsonRequiredStr(j, "model");
        m.serial_number      = jsonRequiredStr(j, "serial_number");
        m.file_version       = jsonRequiredStr(j, "File_Version");
        m.export_date        = jsonRequiredStr(j, "Export_Date");
        m.configuration_hash = jsonRequiredStr(j, "Configuration_Hash");
        // Collect extra: any key not in the known set.
        m.extra = nlohmann::json::object();
        static const std::vector<std::string> known = {
            "machine_name", "manufacturer", "model", "serial_number",
            "File_Version", "Export_Date", "Configuration_Hash"
        };
        for (auto it = j.begin(); it != j.end(); ++it) {
            if (std::find(known.begin(), known.end(), it.key()) == known.end())
                m.extra[it.key()] = it.value();
        }
        return m;
    }

    Machine parseMachine(const HighFive::Group& grp) const {
        Machine m;
        m.id                    = readStr(grp, "ID");
        m.machine_name          = readRequiredStr(grp, "Machine_Name");
        m.manufacturer          = readRequiredStr(grp, "Manufacturer");
        m.model                 = readRequiredStr(grp, "Model");
        m.serial_number         = readRequiredStr(grp, "Serial_Number");
        m.build_plate_x         = readFloat(grp, "Build_Plate_X_Dimension");
        m.build_plate_x_unit    = readUnitLocked(grp, "Build_Plate_X_Dimension_unit", "mm");
        m.build_plate_y         = readFloat(grp, "Build_Plate_Y_Dimension");
        m.build_plate_y_unit    = readUnitLocked(grp, "Build_Plate_Y_Dimension_unit", "mm");
        m.build_plate_z         = readFloat(grp, "Build_Plate_Z_Dimension");
        m.build_plate_z_unit    = readUnitLocked(grp, "Build_Plate_Z_Dimension_unit", "mm");
        m.build_plate_radius    = readFloat(grp, "Build_Plate_Corner_Radius");
        m.build_plate_radius_unit = readUnitLocked(grp, "Build_Plate_Corner_Radius_unit", "mm");
        m.gas_flow_direction    = readStr(grp, "Gas_Flow_Direction");
        m.recoat_direction      = readStr(grp, "Recoat_Direction");
        return m;
    }

    std::vector<OpticalTrain> parseTrains(const HighFive::File& f,
                                           const std::vector<nlohmann::json>& adapted) const {
        auto trains_grp = f.getGroup("Machine/Optical_Trains");
        std::vector<std::string> ids;
        for (const auto& n : trains_grp.listObjectNames()) {
            if (n.size() >= 14 && n.substr(0, 14) == "Optical_Train_")
                ids.push_back(n);
        }
        std::sort(ids.begin(), ids.end());
        std::vector<OpticalTrain> result;
        result.reserve(ids.size());
        for (size_t i = 0; i < ids.size(); ++i) {
            const nlohmann::json* a = (!adapted.empty() && i < adapted.size()) ? &adapted[i] : nullptr;
            result.push_back(parseTrain(f, ids[i], a));
        }
        return result;
    }

    OpticalTrain parseTrain(const HighFive::File& f, const std::string& train_id,
                             const nlohmann::json* adapted) const {
        auto grp = f.getGroup("Machine/Optical_Trains/" + train_id);
        OpticalTrain ot;
        ot.train_id  = train_id;
        // Use adapted JSON when available; fall back to HDF5 for train-level attrs.
        ot.id        = adapted ? jsonStr(*adapted, "ID")                      : readStr(grp, "ID");
        ot.beam_profile_type     = adapted ? jsonStr(*adapted, "Beam_Profile_Type")     : readStr(grp, "Beam_Profile_Type");
        ot.beam_waist_definition = adapted ? jsonStr(*adapted, "Beam_Waist_Definition") : readStr(grp, "Beam_Waist_Definition");
        ot.beam_waist_major      = adapted ? jsonFloat(*adapted, "Beam_Waist_Major")    : readFloat(grp, "Beam_Waist_Major");
        ot.beam_waist_major_unit = adapted ? jsonStr(*adapted, "Beam_Waist_Major_unit")                      : readUnitLocked(grp, "Beam_Waist_Major_unit", "\u03bcm");
        ot.beam_waist_minor      = adapted ? jsonFloat(*adapted, "Beam_Waist_Minor")    : readFloat(grp, "Beam_Waist_Minor");
        ot.beam_waist_minor_unit = adapted ? jsonStr(*adapted, "Beam_Waist_Minor_unit")                      : readUnitLocked(grp, "Beam_Waist_Minor_unit", "\u03bcm");
        ot.beam_waist_offset_z      = adapted ? jsonFloat(*adapted, "Beam_Waist_Offset_Z")      : readFloat(grp, "Beam_Waist_Offset_Z");
        ot.beam_waist_offset_z_unit = adapted ? jsonStr(*adapted, "Beam_Waist_Offset_Z_unit")   : readUnitLocked(grp, "Beam_Waist_Offset_Z_unit", "mm");
        ot.build_plane_offset_major      = adapted ? jsonFloat(*adapted, "Build_Plane_Offset_Major")      : readFloat(grp, "Build_Plane_Offset_Major");
        ot.build_plane_offset_major_unit = adapted ? jsonStr(*adapted, "Build_Plane_Offset_Major_unit")   : readUnitLocked(grp, "Build_Plane_Offset_Major_unit", "mm");
        ot.build_plane_offset_minor      = adapted ? jsonFloat(*adapted, "Build_Plane_Offset_Minor")      : readFloat(grp, "Build_Plane_Offset_Minor");
        ot.build_plane_offset_minor_unit = adapted ? jsonStr(*adapted, "Build_Plane_Offset_Minor_unit")   : readUnitLocked(grp, "Build_Plane_Offset_Minor_unit", "mm");
        ot.collimator_focal_length       = adapted ? jsonFloat(*adapted, "Collimator_Focal_Length")       : readFloat(grp, "Collimator_Focal_Length");
        ot.collimator_focal_length_unit  = adapted ? jsonStr(*adapted, "Collimator_Focal_Length_unit")    : readUnitLocked(grp, "Collimator_Focal_Length_unit", "mm");
        ot.m2_major        = adapted ? jsonFloat(*adapted, "M2_Major")          : readFloat(grp, "M2_Major");
        ot.m2_minor        = adapted ? jsonFloat(*adapted, "M2_Minor")          : readFloat(grp, "M2_Minor");
        ot.major_axis_angle      = adapted ? jsonFloat(*adapted, "Major_Axis_Angle")      : readFloat(grp, "Major_Axis_Angle");
        ot.major_axis_angle_unit = adapted ? jsonStr(*adapted, "Major_Axis_Angle_unit")   : readUnitLocked(grp, "Major_Axis_Angle_unit", "degrees");
        ot.rayleigh_length_major      = adapted ? jsonFloat(*adapted, "Rayleigh_Length_Major")      : readFloat(grp, "Rayleigh_Length_Major");
        ot.rayleigh_length_major_unit = adapted ? jsonStr(*adapted, "Rayleigh_Length_Major_unit")   : readUnitLocked(grp, "Rayleigh_Length_Major_unit", "mm");
        ot.rayleigh_length_minor      = adapted ? jsonFloat(*adapted, "Rayleigh_Length_Minor")      : readFloat(grp, "Rayleigh_Length_Minor");
        ot.rayleigh_length_minor_unit = adapted ? jsonStr(*adapted, "Rayleigh_Length_Minor_unit")   : readUnitLocked(grp, "Rayleigh_Length_Minor_unit", "mm");
        ot.scanner_number         = adapted ? jsonStr(*adapted, "Scanner_Number")             : readStr(grp, "Scanner_Number");
        ot.thermal_lensing_passed = adapted ? jsonBoolFromInt(*adapted, "Thermal_Lensing_Test_Passed") : readBoolFromInt(grp, "Thermal_Lensing_Test_Passed");
        ot.thermal_lensing_focal_plane_shift      = adapted ? jsonFloat(*adapted, "Thermal_Lensing_Focal_Plane_Shift")      : readFloat(grp, "Thermal_Lensing_Focal_Plane_Shift");
        ot.thermal_lensing_focal_plane_shift_unit = adapted ? jsonStr(*adapted, "Thermal_Lensing_Focal_Plane_Shift_unit")   : readUnitLocked(grp, "Thermal_Lensing_Focal_Plane_Shift_unit", "mm");
        ot.thermal_lensing_threshold      = adapted ? jsonFloat(*adapted, "Thermal_Lensing_Threshold")      : readFloat(grp, "Thermal_Lensing_Threshold");
        ot.thermal_lensing_threshold_unit = adapted ? jsonStr(*adapted, "Thermal_Lensing_Threshold_unit")   : readUnitLocked(grp, "Thermal_Lensing_Threshold_unit", "mm");
        ot.scanner      = parseScanner(grp.getGroup("Scanner"));
        ot.light_source = parseLightSource(grp.getGroup("Light_Source"));
        ot.collimator   = parseCollimator(grp.getGroup("Collimator"));
        ot.scanner_card = parseScannerCard(grp.getGroup("Scanner_Card"));
        if (grp.exist("Optional_Components/ClearBox"))
            ot.optional_components.clearbox = parseClearBox(
                grp.getGroup("Optional_Components/ClearBox"));
        if (grp.exist("scan_field_correction_file"))
            ot.scan_field_correction_file = parseSfcf(
                grp.getDataSet("scan_field_correction_file"));
        return ot;
    }

    AxisConfig parseAxis(const HighFive::Group& grp) const {
        AxisConfig ax;
        ax.actual_bit_resolution      = readInt(grp, "Actual_Bit_Resolution");
        ax.actual_bit_resolution_unit = readStr(grp, "Actual_Bit_Resolution_unit");
        ax.commanded_bit_resolution      = readInt(grp, "Commanded_Bit_Resolution");
        ax.commanded_bit_resolution_unit = readStr(grp, "Commanded_Bit_Resolution_unit");
        ax.control_type         = readStr(grp, "Control_Type");
        ax.range_of_motion      = readFloat(grp, "Range_Of_Motion");
        ax.range_of_motion_unit = readStr(grp, "Range_Of_Motion_unit");
        ax.smoothing_kernel     = readStr(grp, "Smoothing_Kernel");
        ax.smoothing_parameters = readFloat(grp, "Smoothing_Parameters");
        ax.tuning_parameters    = readStr(grp, "Tuning_Parameters");
        ax.tuning_type          = readStr(grp, "Tuning_Type");
        return ax;
    }

    Scanner parseScanner(const HighFive::Group& grp) const {
        Scanner s;
        s.manufacturer  = readRequiredStr(grp, "Manufacturer");
        s.model         = readRequiredStr(grp, "Model");
        s.serial_number = readStr(grp, "Serial_Number").value_or("");
        s.working_distance      = readFloat(grp, "Working_Distance");
        s.working_distance_unit = readUnitLocked(grp, "Working_Distance_unit", "mm");
        s.scan_field_x      = readFloat(grp, "Scan_Field_Size_X");
        s.scan_field_x_unit = readUnitLocked(grp, "Scan_Field_Size_X_unit", "mm");
        s.scan_field_y      = readFloat(grp, "Scan_Field_Size_Y");
        s.scan_field_y_unit = readUnitLocked(grp, "Scan_Field_Size_Y_unit", "mm");
        s.scan_field_z      = readFloat(grp, "Scan_Field_Size_Z");
        s.scan_field_z_unit = readUnitLocked(grp, "Scan_Field_Size_Z_unit", "mm");
        s.scan_head_offset_x      = readFloat(grp, "Scan_Head_Offset_X");
        s.scan_head_offset_x_unit = readUnitLocked(grp, "Scan_Head_Offset_X_unit", "mm");
        s.scan_head_offset_y      = readFloat(grp, "Scan_Head_Offset_Y");
        s.scan_head_offset_y_unit = readUnitLocked(grp, "Scan_Head_Offset_Y_unit", "mm");
        s.scan_head_offset_z      = readFloat(grp, "Scan_Head_Offset_Z");
        s.scan_head_offset_z_unit = readUnitLocked(grp, "Scan_Head_Offset_Z_unit", "mm");
        s.scan_head_rotation      = readFloat(grp, "Scan_Head_Rotation");
        s.scan_head_rotation_unit = readUnitLocked(grp, "Scan_Head_Rotation_unit", "degrees");
        s.axis_configuration = readStr(grp, "Axis_Configuration");
        s.x_axis = parseAxis(grp.getGroup("X_Axis"));
        s.y_axis = parseAxis(grp.getGroup("Y_Axis"));
        if (grp.exist("Z_Axis")) s.z_axis = parseAxis(grp.getGroup("Z_Axis"));
        if (grp.exist("Focus"))  s.focus  = parseAxis(grp.getGroup("Focus"));
        return s;
    }

    LightSource parseLightSource(const HighFive::Group& grp) const {
        LightSource ls;
        ls.manufacturer  = readRequiredStr(grp, "Manufacturer");
        ls.model         = readRequiredStr(grp, "Model");
        ls.serial_number = readRequiredStr(grp, "Serial_Number");
        ls.wavelength      = readFloat(grp, "Light_Wavelength");
        ls.wavelength_unit = readUnitLocked(grp, "Light_Wavelength_unit", "nm");
        ls.power_max_nominal      = readFloat(grp, "Power_Max_Nominal");
        ls.power_max_nominal_unit = readUnitLocked(grp, "Power_Max_Nominal_unit", "W");
        ls.power_max_actual      = readFloat(grp, "Power_Max_Actual");
        ls.power_max_actual_unit = readUnitLocked(grp, "Power_Max_Actual_unit", "W");
        ls.power_min_actual      = readFloat(grp, "Power_Min_Actual");
        ls.power_min_actual_unit = readUnitLocked(grp, "Power_Min_Actual_unit", "W");
        ls.power_min_nominal      = readFloat(grp, "Power_Min_Nominal");
        ls.power_min_nominal_unit = readUnitLocked(grp, "Power_Min_Nominal_unit", "W");
        ls.power_bit_resolution      = readFloat(grp, "Power_Bit_Resolution");
        ls.power_bit_resolution_unit = readUnitLocked(grp, "Power_Bit_Resolution_unit", "bits");
        ls.watts_to_volts_algorithm = readStr(grp, "Watts_To_Volts_Algorithm");
        ls.watts_to_volts_params    = readStr(grp, "Watts_To_Volts_Params");
        return ls;
    }

    Collimator parseCollimator(const HighFive::Group& grp) const {
        Collimator c;
        c.manufacturer  = readRequiredStr(grp, "Manufacturer");
        c.model         = readRequiredStr(grp, "Model");
        c.serial_number = readRequiredStr(grp, "Serial_Number");
        c.focal_length      = readFloat(grp, "Focal_Length");
        c.focal_length_unit = readUnitLocked(grp, "Focal_Length_unit", "mm");
        return c;
    }

    ScannerCard parseScannerCard(const HighFive::Group& grp) const {
        ScannerCard sc;
        sc.manufacturer  = readRequiredStr(grp, "Manufacturer");
        sc.model         = readRequiredStr(grp, "Model");
        sc.serial_number = readRequiredStr(grp, "Serial_Number");
        sc.communication_protocol = readStr(grp, "Communication_Protocol");
        sc.sample_period      = readFloat(grp, "Sample_Period");
        sc.sample_period_unit = readUnitLocked(grp, "Sample_Period_unit", "\u03bcs");
        return sc;
    }

    // Scalar fields only; correction_data / inverse_correction_data deferred to \u00a74.11.
    ClearBox parseClearBox(const HighFive::Group& grp) const {
        ClearBox cb;
        cb.ip_address              = readRequiredStr(grp, "Ip_Address");
        cb.serial_number           = readStr(grp, "Serial_Number");
        cb.data_port               = readInt(grp, "Data_Port");
        cb.server_port             = readInt(grp, "Server_Port");
        cb.actual_timing_offset    = readInt(grp, "Actual_Timing_Offset");
        cb.commanded_timing_offset = readInt(grp, "Commanded_Timing_Offset");
        cb.manufacturer            = readStr(grp, "Manufacturer");
        cb.model                   = readStr(grp, "Model");
        cb.output_path             = readStr(grp, "Output_Path");
        cb.selected_camera         = readStr(grp, "Selected_Camera");
        cb.custom_video_format     = readStr(grp, "Custom_Video_Format");
        cb.video_output            = readStr(grp, "Video_Output");
        cb.show_console            = readBoolFromInt(grp, "Show_Console");
        cb.software_trigger_delay  = readInt(grp, "Software_Trigger_Delay");
        cb.volts_to_watts_algorithm     = readStr(grp, "Volts_To_Watts_Algorithm");
        cb.volts_to_watts_params        = readStr(grp, "Volts_To_Watts_Params");
        cb.correction_grid_domain_shape = readStr(grp, "Correction_Grid_Domain_Shape");
        cb.inverse_grid_domain_shape    = readStr(grp, "Inverse_Grid_Domain_Shape");
        return cb;
    }

    // Scalar attrs only; raw_bytes deferred to \u00a74.11.
    ScanFieldCorrectionFile parseSfcf(const HighFive::DataSet& ds) const {
        ScanFieldCorrectionFile s;
        s.document_name    = readRequiredStr(ds, "document_name");
        s.document_id      = readRequiredStr(ds, "document_id");
        s.file_size        = readInt(ds, "file_size").value_or(0);
        s.valid_as_of_date = readRequiredStr(ds, "valid_as_of_date");
        s.document_created_at = readStr(ds, "document_created_at");
        s.document_type       = readStr(ds, "document_type");
        s.original_uri        = readStr(ds, "original_uri");
        return s;
    }
    std::optional<OpcuaConfig> parseOpcua(const HighFive::File& f) const {
        if (!f.exist("OPCUA")) return std::nullopt;

        auto client_grp = f.getGroup("OPCUA/Client");
        OpcuaClientConfig client;
        client.server_url        = readRequiredStr(client_grp, "Server_URL");
        client.auth_mode         = readRequiredStr(client_grp, "Auth_Mode");
        client.security_mode     = readRequiredStr(client_grp, "Security_Mode");
        client.security_policy   = readRequiredStr(client_grp, "Security_Policy");
        client.bfs_max_depth     = readInt(client_grp, "BFS_Max_Depth").value_or(0);
        client.publish_interval  = readInt(client_grp, "Publish_Interval").value_or(0);
        client.sampling_interval = readInt(client_grp, "Sampling_Interval").value_or(0);
        client.session_timeout   = readInt(client_grp, "Session_Timeout").value_or(0);
        client.extra = collectExtra(client_grp, {
            "Server_URL", "Auth_Mode", "Security_Mode", "Security_Policy",
            "BFS_Max_Depth", "Publish_Interval", "Sampling_Interval", "Session_Timeout"
        });

        auto pipe_grp = f.getGroup("OPCUA/Pipe");
        OpcuaPipeConfig pipe;
        pipe.pipe_enabled = readBoolFromInt(pipe_grp, "Pipe_Enabled").value_or(false);
        pipe.buffer_size  = readInt(pipe_grp, "Buffer_Size").value_or(0);
        pipe.extra = collectExtra(pipe_grp, {"Pipe_Enabled", "Buffer_Size"});

        auto tgrp = f.getGroup("OPCUA/Triggers");
        OpcuaConfig opcua;
        opcua.triggers_enabled = readBoolFromInt(tgrp, "Triggers_Enabled");
        for (const auto& name : tgrp.listObjectNames()) {
            auto tg = tgrp.getGroup(name);
            OpcuaTrigger t;
            t.id           = readStr(tg, "ID");
            t.signal       = readStr(tg, "Signal");
            t.subsystem    = readStr(tg, "Subsystem");
            t.rule_enabled = readBoolFromInt(tg, "Rule_Enabled");
            t.start_value  = readStr(tg, "Start_Value");
            t.stop_value   = readStr(tg, "Stop_Value");
            t.extra = collectExtra(tg, {
                "ID", "Signal", "Subsystem", "Rule_Enabled", "Start_Value", "Stop_Value"
            });
            opcua.triggers[name] = std::move(t);
        }
        opcua.client = std::move(client);
        opcua.pipe   = std::move(pipe);
        return opcua;
    }

    // Return the N-th sorted Optical_Train_NN key (0-based).
    std::string trainIdAt(const HighFive::File& f, size_t train_index) const {        auto trains_grp = f.getGroup("Machine/Optical_Trains");
        std::vector<std::string> ids;
        for (const auto& n : trains_grp.listObjectNames())
            if (n.size() >= 14 && n.substr(0, 14) == "Optical_Train_")
                ids.push_back(n);
        std::sort(ids.begin(), ids.end());
        if (train_index >= ids.size())
            throw std::out_of_range(
                "train_index " + std::to_string(train_index) +
                " out of range [0, " + std::to_string(ids.size()) + ")");
        return ids[train_index];
    }

    // Read a 3-D float64 dataset into a flat CorrectionData buffer.
    CorrectionData readCorrectionGrid(const HighFive::File& f,
                                      const std::string& ds_path) const {
        auto ds   = f.getDataSet(ds_path);
        auto dims = ds.getSpace().getDimensions();
        CorrectionData cd;
        cd.shape = {dims[0], dims[1], dims[2]};
        cd.data.resize(dims[0] * dims[1] * dims[2]);
        H5Dread(ds.getId(), H5T_NATIVE_DOUBLE, H5S_ALL, H5S_ALL, H5P_DEFAULT,
                cd.data.data());
        return cd;
    }

    // Convert a flat CorrectionData buffer to a nested Grid3D; NaN → nullopt.
    static Grid3D correctionDataToGrid3D(const CorrectionData& cd) {
        auto d0 = cd.shape[0], d1 = cd.shape[1], d2 = cd.shape[2];
        Grid3D grid(d0, std::vector<std::vector<GridCell>>(d1, std::vector<GridCell>(d2)));
        for (size_t i = 0; i < d0; ++i)
            for (size_t j = 0; j < d1; ++j)
                for (size_t k = 0; k < d2; ++k) {
                    double v = cd.data[i * d1 * d2 + j * d2 + k];
                    grid[i][j][k] = std::isnan(v) ? GridCell{} : GridCell{v};
                }
        return grid;
    }};

} // namespace machine_config
