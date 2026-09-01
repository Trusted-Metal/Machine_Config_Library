#pragma once
// File_Version 1.1 HDF5 reader — on-disk layout, attribute names, and
// casting. Public MachineConfigReader peeks File_Version then dispatches
// here.
//
// Deliberately independent of any other version's own reader header — every
// helper used here (type-conversion, sub-group parsing) is defined fresh in
// this file, even where the on-disk shape happens to match a prior version
// today (Collimator, Scanner_Card, the scan-field-correction-file dataset,
// TM_OPCUA's inner Client/Pipe/Triggers shape, train-attribute reading, and
// axis reading minus tuning). This includes the low-level attribute-reading
// helpers themselves (readStr/readFloat/readInt/readBoolFromInt/
// readUnitLocked/readRequiredStr/collectExtra): a sibling version's own
// reader header happens to declare equivalents of these in the shared
// `machine_config` namespace, but they are physically defined *inside* that
// other header, so including it to reuse them would still be exactly the
// forbidden coupling. Everything here is scoped under this file's own
// namespace instead — see cpp/tests/test_version_adapter_isolation.cpp,
// which scans for cross-version references and fails the build if this file
// ever reaches back into another version's code.
//
// This also sidesteps a real compile hazard: a sibling version's reader
// declares its own free-function template overloads of these same names
// directly in the bare `machine_config` namespace. If this file did the
// same, any translation unit that included both headers (e.g. reader.hpp,
// which must include every version to build its dispatch registry) would
// see two definitions of the same template in the same scope — a hard
// redefinition error, not just an ODR risk. Namespacing everything under
// `capabilities::v1_1` avoids that entirely.

#include "machine_config/adapters.hpp"
#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_1/compound_types.hpp"
#include "machine_config/capabilities/v1_1/layout.hpp"

#include <highfive/H5File.hpp>

#include <algorithm>
#include <cctype>
#include <filesystem>
#include <iomanip>
#include <map>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

namespace machine_config::capabilities::v1_1 {

// ---------------------------------------------------------------------------
// Attribute-reading helpers — independent copies; see file header comment.
// ---------------------------------------------------------------------------

using RawAttr = std::variant<std::string, int64_t, double>;

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
            // throws length_error. Fix: read VarLen attrs using the file's own type
            // directly; HDF5 always allocates a null-terminated heap pointer for VarLen
            // data regardless of the strpad metadata.
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
                size_t sz = H5Tget_size(dtype_id);
                std::vector<char> buf(sz + 1, '\0');
                H5Aread(attr.getId(), dtype_id, buf.data());
                result = buf.data();
            }
            return RawAttr{std::move(result)};
        }
    }
}

static inline std::string rawToString(const RawAttr& raw) {
    return std::visit([](auto&& v) -> std::string {
        using T = std::decay_t<decltype(v)>;
        if constexpr (std::is_same_v<T, std::string>) return v;
        else return std::to_string(v);
    }, raw);
}

template <typename Loc>
inline std::string readRequiredStr(const Loc& loc, const std::string& key) {
    auto raw = readRaw(loc, key);
    if (!raw) return {};
    return rawToString(*raw);
}

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

template <typename Loc>
inline std::optional<bool> readBoolFromInt(const Loc& loc, const std::string& key) {
    auto v = readInt(loc, key);
    if (!v) return std::nullopt;
    if (*v == 0) return false;
    if (*v == 1) return true;
    throw std::runtime_error(
        "attribute '" + key + "' has value " + std::to_string(*v) + "; expected 0 or 1 (Rule 8).");
}

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
// Hdf5AdapterV1_1
// ---------------------------------------------------------------------------

class Hdf5AdapterV1_1 : public ReaderAdapter {
public:
    explicit Hdf5AdapterV1_1(std::filesystem::path path)
        : path_(std::move(path)) {}

    MachineConfig parse() const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        return parseInner(f);
    }

    MachineConfig parseWithBinary() const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto cfg = parseInner(f);
        for (size_t i = 0; i < cfg.optical_trains.size(); ++i) {
            auto& ot  = cfg.optical_trains[i];
            auto  tid = trainIdAt(f, i);
            if (ot.optional_components.clearbox && f.exist(clearboxPathById(tid))) {
                auto cd  = readCorrectionGrid(f, correctionDataPathById(tid));
                auto icd = readCorrectionGrid(f, inverseCorrectionDataPathById(tid));
                ot.optional_components.clearbox->correction_data         = detail::correctionDataToGrid3D(cd);
                ot.optional_components.clearbox->inverse_correction_data = detail::correctionDataToGrid3D(icd);
            }
            if (ot.scan_field_correction_file) {
                auto ds   = f.getDataSet(scanFieldCorrectionFilePathById(tid));
                auto dims = ds.getSpace().getDimensions();
                std::vector<uint8_t> bytes(dims[0]);
                H5Dread(ds.getId(), H5T_NATIVE_UINT8, H5S_ALL, H5S_ALL, H5P_DEFAULT, bytes.data());
                ot.scan_field_correction_file->raw_bytes = std::move(bytes);
            }
        }
        return cfg;
    }

    std::string toJson(int indent = 2, bool /*include_binary*/ = false) const override {
        nlohmann::json j = parse();
        return j.dump(indent);
    }

    nlohmann::json getRawGroup(const std::string& hdf5_path) const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        try {
            return collectExtra(f.getGroup(hdf5_path), {});
        } catch (...) {
            return nlohmann::json::object();
        }
    }

    CorrectionData getCorrectionData(size_t train_index) const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        return readCorrectionGrid(f, correctionDataPathById(tid));
    }

    CorrectionData getInverseCorrectionData(size_t train_index) const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        return readCorrectionGrid(f, inverseCorrectionDataPathById(tid));
    }

    std::vector<uint8_t> getScanFieldCorrectionBytes(size_t train_index) const override {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        auto tid = trainIdAt(f, train_index);
        auto ds  = f.getDataSet(scanFieldCorrectionFilePathById(tid));
        auto dims = ds.getSpace().getDimensions();
        std::vector<uint8_t> bytes(dims[0]);
        H5Dread(ds.getId(), H5T_NATIVE_UINT8, H5S_ALL, H5S_ALL, H5P_DEFAULT, bytes.data());
        return bytes;
    }

private:
    std::filesystem::path path_;

    MachineConfig parseInner(const HighFive::File& f) const {
        MachineConfig cfg;
        cfg.meta    = parseMeta(f);
        cfg.machine = parseMachine(f.getGroup(ROOT_MACHINE));

        // Change 1 (Consolidate): one shared Output_Path/Software_Trigger_Delay
        // value, read once from Extensions/ClearBox/ itself.
        bool hasClearboxRoot = f.exist(GROUP_CLEARBOX);
        std::optional<std::string> sharedOutputPath;
        std::optional<int64_t> sharedSoftwareTriggerDelay;
        if (hasClearboxRoot) {
            auto cbRoot = f.getGroup(GROUP_CLEARBOX);
            sharedOutputPath           = readStr(cbRoot, "Output_Path");
            sharedSoftwareTriggerDelay = readInt(cbRoot, "Software_Trigger_Delay");
        }

        auto trainsGrp = f.getGroup(ROOT_OPTICAL_TRAINS);
        std::vector<std::string> ids;
        const std::string prefix = TRAIN_ID_PREFIX;
        for (const auto& n : trainsGrp.listObjectNames())
            if (n.size() >= prefix.size() && n.compare(0, prefix.size(), prefix) == 0)
                ids.push_back(n);
        std::sort(ids.begin(), ids.end());

        std::vector<OpticalTrain> trains;
        trains.reserve(ids.size());
        for (const auto& tid : ids)
            trains.push_back(parseTrain(f, tid, hasClearboxRoot, sharedOutputPath, sharedSoftwareTriggerDelay));
        cfg.optical_trains = std::move(trains);

        cfg.opcua = parseOpcua(f);
        return cfg;
    }

    MachineConfigMeta parseMeta(const HighFive::File& f) const {
        MachineConfigMeta m;
        m.schema_version     = "v1";
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

    Machine parseMachine(const HighFive::Group& grp) const {
        Machine m;
        m.id                      = readStr(grp, "ID");
        m.machine_name            = readRequiredStr(grp, "Machine_Name");
        m.manufacturer            = readRequiredStr(grp, "Manufacturer");
        m.model                   = readRequiredStr(grp, "Model");
        m.serial_number           = readRequiredStr(grp, "Serial_Number");
        m.build_plate_x           = readFloat(grp, "Build_Plate_X_Dimension");
        m.build_plate_x_unit      = readUnitLocked(grp, "Build_Plate_X_Dimension_unit", "mm");
        m.build_plate_y           = readFloat(grp, "Build_Plate_Y_Dimension");
        m.build_plate_y_unit      = readUnitLocked(grp, "Build_Plate_Y_Dimension_unit", "mm");
        m.build_plate_z           = readFloat(grp, "Build_Plate_Z_Dimension");
        m.build_plate_z_unit      = readUnitLocked(grp, "Build_Plate_Z_Dimension_unit", "mm");
        m.build_plate_radius      = readFloat(grp, "Build_Plate_Corner_Radius");
        m.build_plate_radius_unit = readUnitLocked(grp, "Build_Plate_Corner_Radius_unit", "mm");
        m.gas_flow_direction      = readStr(grp, "Gas_Flow_Direction");
        m.recoat_direction        = readStr(grp, "Recoat_Direction");
        return m;
    }

    OpticalTrain parseTrain(
        const HighFive::File& f,
        const std::string& train_id,
        bool hasClearboxRoot,
        const std::optional<std::string>& sharedOutputPath,
        const std::optional<int64_t>& sharedSoftwareTriggerDelay) const
    {
        auto grp = f.getGroup(trainPathById(train_id));
        OpticalTrain ot;
        ot.train_id  = train_id;
        ot.id        = readStr(grp, "ID");
        ot.beam_profile_type     = readStr(grp, "Beam_Profile_Type");
        ot.beam_waist_definition = readStr(grp, "Beam_Waist_Definition");
        ot.beam_waist_major      = readFloat(grp, "Beam_Waist_Major");
        ot.beam_waist_major_unit = readUnitLocked(grp, "Beam_Waist_Major_unit", "μm");
        ot.beam_waist_minor      = readFloat(grp, "Beam_Waist_Minor");
        ot.beam_waist_minor_unit = readUnitLocked(grp, "Beam_Waist_Minor_unit", "μm");
        ot.beam_waist_offset_z      = readFloat(grp, "Beam_Waist_Offset_Z");
        ot.beam_waist_offset_z_unit = readUnitLocked(grp, "Beam_Waist_Offset_Z_unit", "mm");
        ot.build_plane_offset_major      = readFloat(grp, "Build_Plane_Offset_Major");
        ot.build_plane_offset_major_unit = readUnitLocked(grp, "Build_Plane_Offset_Major_unit", "mm");
        ot.build_plane_offset_minor      = readFloat(grp, "Build_Plane_Offset_Minor");
        ot.build_plane_offset_minor_unit = readUnitLocked(grp, "Build_Plane_Offset_Minor_unit", "mm");
        ot.collimator_focal_length      = readFloat(grp, "Collimator_Focal_Length");
        ot.collimator_focal_length_unit = readUnitLocked(grp, "Collimator_Focal_Length_unit", "mm");
        ot.m2_major        = readFloat(grp, "M2_Major");
        ot.m2_minor        = readFloat(grp, "M2_Minor");
        ot.major_axis_angle      = readFloat(grp, "Major_Axis_Angle");
        ot.major_axis_angle_unit = readUnitLocked(grp, "Major_Axis_Angle_unit", "degrees");
        ot.rayleigh_length_major      = readFloat(grp, "Rayleigh_Length_Major");
        ot.rayleigh_length_major_unit = readUnitLocked(grp, "Rayleigh_Length_Major_unit", "mm");
        ot.rayleigh_length_minor      = readFloat(grp, "Rayleigh_Length_Minor");
        ot.rayleigh_length_minor_unit = readUnitLocked(grp, "Rayleigh_Length_Minor_unit", "mm");
        ot.scanner_number         = readStr(grp, "Scanner_Number");
        ot.thermal_lensing_passed = readBoolFromInt(grp, "Thermal_Lensing_Test_Passed");
        ot.thermal_lensing_focal_plane_shift      = readFloat(grp, "Thermal_Lensing_Focal_Plane_Shift");
        ot.thermal_lensing_focal_plane_shift_unit = readUnitLocked(grp, "Thermal_Lensing_Focal_Plane_Shift_unit", "mm");
        ot.thermal_lensing_threshold      = readFloat(grp, "Thermal_Lensing_Threshold");
        ot.thermal_lensing_threshold_unit = readUnitLocked(grp, "Thermal_Lensing_Threshold_unit", "mm");
        ot.scanner      = parseScanner(f.getGroup(scannerPathById(train_id)));
        ot.light_source = parseLightSource(f.getGroup(lightSourcePathById(train_id)));
        ot.collimator   = parseCollimator(f.getGroup(collimatorPathById(train_id)));
        ot.scanner_card = parseScannerCard(f.getGroup(scannerCardPathById(train_id)));

        if (hasClearboxRoot) {
            std::string cbPath = clearboxPathById(train_id);
            if (f.exist(cbPath))
                ot.optional_components.clearbox = parseClearBox(f, train_id, sharedOutputPath, sharedSoftwareTriggerDelay);
        }

        std::string sfcfPath = scanFieldCorrectionFilePathById(train_id);
        if (f.exist(sfcfPath))
            ot.scan_field_correction_file = parseSfcf(f.getDataSet(sfcfPath));

        return ot;
    }

    // Change 5: Tuning_Parameters/Tuning_Type have no on-disk source in this
    // File_Version — always nullopt here, regardless of what's in the group.
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
        ax.tuning_parameters    = std::nullopt;
        ax.tuning_type          = std::nullopt;
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
        s.invert_actual_x = readBoolFromInt(grp, "Invert_Actual_X").value_or(false);
        s.invert_actual_y = readBoolFromInt(grp, "Invert_Actual_Y").value_or(false);
        s.invert_commanded_x = readBoolFromInt(grp, "Invert_Commanded_X").value_or(false);
        s.invert_commanded_y = readBoolFromInt(grp, "Invert_Commanded_Y").value_or(false);
        return s;
    }

    // Trims a fixed-length char buffer at its first NUL byte.
    static std::string fixedBufToString(const char* buf, std::size_t n) {
        std::size_t len = 0;
        while (len < n && buf[len] != '\0') ++len;
        return std::string(buf, len);
    }

    // Reads a Derivation_Equation_Constants-shaped dataset by name,
    // defaulting to an empty (non-nil) vector if the dataset is absent.
    // Shared between Power_Characterization and SynchronousSensor, which use
    // the identical row shape under different dataset names.
    std::vector<EquationConstant> readEquationConstants(const HighFive::Group& grp, const std::string& dsName) const {
        std::vector<EquationConstant> out;
        if (!grp.exist(dsName)) return out;
        std::vector<V1_1EquationConstantRow> rows;
        grp.getDataSet(dsName).read(rows);
        out.reserve(rows.size());
        for (const auto& r : rows)
            out.push_back({fixedBufToString(r.name, V1_1EquationConstantMaxNameBytes), r.value});
        return out;
    }

    // Reads a Calibration_Points-shaped dataset by name, defaulting to an
    // empty (non-nil) vector if the dataset is absent.
    std::vector<CalibrationPoint> readCalibrationPoints(const HighFive::Group& grp, const std::string& dsName) const {
        std::vector<CalibrationPoint> out;
        if (!grp.exist(dsName)) return out;
        std::vector<V1_1CalibrationPointRow> rows;
        grp.getDataSet(dsName).read(rows);
        out.reserve(rows.size());
        for (const auto& r : rows)
            out.push_back({r.input_value, r.output_value});
        return out;
    }

    // Change 3/4: shared Power_Characterization shape at two different
    // on-disk paths (ClearBox's and Light_Source's).
    PowerCharacterization parsePowerCharacterization(const HighFive::Group& grp) const {
        PowerCharacterization pc;
        pc.algorithm_type          = readStr(grp, "Algorithm_Type");
        pc.algorithm_equation      = readStr(grp, "Algorithm_Equation");
        pc.input_type              = readStr(grp, "Input_Type");
        pc.units_derived_quantity  = readStr(grp, "Units_Derived_Quantity");
        pc.derivation_equation_constants = readEquationConstants(grp, DS_DERIVATION_EQUATION_CONSTANTS);
        pc.characterization_points       = readCalibrationPoints(grp, DS_CHARACTERIZATION_POINTS);
        return pc;
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
        // Change 4: no on-disk source in this File_Version — superseded by
        // power_characterization.
        ls.watts_to_volts_algorithm = std::nullopt;
        ls.watts_to_volts_params    = std::nullopt;
        if (grp.exist(GROUP_POWER_CHARACTERIZATION))
            ls.power_characterization = parsePowerCharacterization(grp.getGroup(GROUP_POWER_CHARACTERIZATION));
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
        sc.sample_period_unit = readUnitLocked(grp, "Sample_Period_unit", "μs");
        return sc;
    }

    // Change 1: relocated to Extensions/ClearBox/<train_id>/, several attrs
    // dropped (Selected_Camera, Custom_Video_Format, Video_Output,
    // Show_Console, Correction_Grid_Domain_Shape, Inverse_Grid_Domain_Shape —
    // always nullopt here), Firmware_Version added, Output_Path/
    // Software_Trigger_Delay Consolidated (passed in already resolved).
    ClearBox parseClearBox(
        const HighFive::File& f,
        const std::string& train_id,
        const std::optional<std::string>& sharedOutputPath,
        const std::optional<int64_t>& sharedSoftwareTriggerDelay) const
    {
        auto grp = f.getGroup(clearboxPathById(train_id));
        ClearBox cb;
        cb.ip_address              = readRequiredStr(grp, "Ip_Address");
        cb.serial_number           = readStr(grp, "Serial_Number");
        cb.data_port               = readInt(grp, "Data_Port");
        cb.server_port             = readInt(grp, "Server_Port");
        cb.actual_timing_offset    = readInt(grp, "Actual_Timing_Offset");
        cb.commanded_timing_offset = readInt(grp, "Commanded_Timing_Offset");
        cb.manufacturer            = readStr(grp, "Manufacturer");
        cb.model                   = readStr(grp, "Model");
        cb.output_path             = sharedOutputPath;
        cb.selected_camera         = std::nullopt;
        cb.custom_video_format     = std::nullopt;
        cb.video_output            = std::nullopt;
        cb.show_console            = std::nullopt;
        cb.software_trigger_delay  = sharedSoftwareTriggerDelay;
        cb.volts_to_watts_algorithm     = std::nullopt;
        cb.volts_to_watts_params        = std::nullopt;
        cb.correction_grid_domain_shape = std::nullopt;
        cb.inverse_grid_domain_shape    = std::nullopt;
        cb.synchronous_sensors          = parseSynchronousSensors(grp);
        cb.firmware_version             = readStr(grp, "Firmware_Version");
        if (grp.exist(GROUP_POWER_CHARACTERIZATION))
            cb.power_characterization = parsePowerCharacterization(grp.getGroup(GROUP_POWER_CHARACTERIZATION));
        return cb;
    }

    SynchronousSensor parseSynchronousSensor(const HighFive::Group& grp) const {
        SynchronousSensor s;
        s.enabled                    = readBoolFromInt(grp, "Enabled");
        s.sensor_name                = readStr(grp, "Sensor_Name");
        s.sensor_output_range_low    = readFloat(grp, "Sensor_Output_Range_Low");
        s.sensor_output_range_high   = readFloat(grp, "Sensor_Output_Range_High");
        s.sensor_output_space        = readStr(grp, "Sensor_Output_Space");
        s.sensor_model               = readStr(grp, "Sensor_Model");
        s.sensor_manufacturer        = readStr(grp, "Sensor_Manufacturer");
        s.sensor_scope               = readStr(grp, "Sensor_Scope");
        s.units_derived_quantity     = readStr(grp, "Units_Derived_Quantity");
        s.port_id                    = readInt(grp, "Port_ID");
        s.sensor_type                = readStr(grp, "Sensor_Type");
        s.input_type                 = readStr(grp, "Input_Type");
        s.algorithm_type             = readStr(grp, "Algorithm_Type");
        s.algorithm_equation         = readStr(grp, "Algorithm_Equation");
        s.calibration_source         = readStr(grp, "Calibration_Source");
        s.calibration_verified       = readBoolFromInt(grp, "Calibration_Verified");
        s.sample_period              = readFloat(grp, "Sample_Period");
        s.metadata                   = readStr(grp, "Metadata");
        s.derivation_equation_constants = readEquationConstants(grp, "Derivation_Equation_Constants");
        s.calibration_points            = readCalibrationPoints(grp, "Calibration_Points");
        return s;
    }

    std::map<std::string, SynchronousSensor> parseSynchronousSensors(const HighFive::Group& grp) const {
        std::map<std::string, SynchronousSensor> sensors;
        if (!grp.exist("Synchronous_Sensors")) return sensors;
        auto sensors_grp = grp.getGroup("Synchronous_Sensors");
        for (const auto& name : sensors_grp.listObjectNames())
            sensors[name] = parseSynchronousSensor(sensors_grp.getGroup(name));
        return sensors;
    }

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

    // Change 2: relocated verbatim to Extensions/TM_OPCUA/ — nothing inside
    // the group's own shape changes.
    std::optional<OpcuaConfig> parseOpcua(const HighFive::File& f) const {
        if (!f.exist(ROOT_OPCUA)) return std::nullopt;

        auto client_grp = f.getGroup(OPCUA_CLIENT);
        OpcuaClientConfig client;
        client.server_url        = readRequiredStr(client_grp, "Server_URL");
        client.auth_mode         = readRequiredStr(client_grp, "Auth_Mode");
        client.security_mode     = readRequiredStr(client_grp, "Security_Mode");
        client.security_policy   = readRequiredStr(client_grp, "Security_Policy");
        client.bfs_max_depth     = readInt(client_grp, "BFS_Max_Depth").value_or(0);
        client.publish_interval  = readInt(client_grp, "Publish_Interval").value_or(0);
        client.sampling_interval = readInt(client_grp, "Sampling_Interval").value_or(0);
        client.session_timeout   = readInt(client_grp, "Session_Timeout").value_or(0);
        client.keep_alive_count             = readInt(client_grp, "Keep_Alive_Count");
        client.lifetime_count               = readInt(client_grp, "Lifetime_Count");
        client.machine_profile              = readStr(client_grp, "Machine_Profile");
        client.queue_policy                 = readStr(client_grp, "Queue_Policy");
        client.queue_size_data_change       = readInt(client_grp, "Queue_Size_Data_Change");
        client.queue_size_events            = readInt(client_grp, "Queue_Size_Events");
        client.reconnect_interval           = readInt(client_grp, "Reconnect_Interval");
        client.root_node                    = readStr(client_grp, "Root_Node");
        client.sync_loop_interval_initial   = readInt(client_grp, "Sync_Loop_Interval_Initial");
        client.sync_loop_interval_settled   = readInt(client_grp, "Sync_Loop_Interval_Settled");
        client.extra = collectExtra(client_grp, {
            "Server_URL", "Auth_Mode", "Security_Mode", "Security_Policy",
            "BFS_Max_Depth", "Publish_Interval", "Sampling_Interval", "Session_Timeout",
            "Keep_Alive_Count", "Lifetime_Count", "Machine_Profile", "Queue_Policy",
            "Queue_Size_Data_Change", "Queue_Size_Events", "Reconnect_Interval", "Root_Node",
            "Sync_Loop_Interval_Initial", "Sync_Loop_Interval_Settled"
        });

        auto pipe_grp = f.getGroup(OPCUA_PIPE);
        OpcuaPipeConfig pipe;
        pipe.pipe_enabled = readBoolFromInt(pipe_grp, "Pipe_Enabled").value_or(false);
        pipe.buffer_size  = readInt(pipe_grp, "Buffer_Size").value_or(0);
        pipe.configure_client         = readBoolFromInt(pipe_grp, "Configure_Client");
        pipe.inbound_rate_limit       = readInt(pipe_grp, "Inbound_Rate_Limit");
        pipe.max_inbound_message_size = readInt(pipe_grp, "Max_Inbound_Message_Size");
        pipe.min_integrity_level      = readStr(pipe_grp, "Min_Integrity_Level");
        pipe.pipe_name                = readStr(pipe_grp, "Pipe_Name");
        pipe.user_access_level        = readStr(pipe_grp, "User_Access_Level");
        pipe.extra = collectExtra(pipe_grp, {
            "Pipe_Enabled", "Buffer_Size", "Configure_Client", "Inbound_Rate_Limit",
            "Max_Inbound_Message_Size", "Min_Integrity_Level", "Pipe_Name", "User_Access_Level"
        });

        auto tgrp = f.getGroup(OPCUA_TRIGGERS);
        OpcuaConfig opcua;
        opcua.triggers_enabled = readBoolFromInt(tgrp, "Triggers_Enabled");
        opcua.trigger_stop_ceiling_layers = readInt(tgrp, "Trigger_Stop_Ceiling_Layers");
        for (const auto& name : tgrp.listObjectNames()) {
            auto tg = tgrp.getGroup(name);
            OpcuaTrigger t;
            t.id           = readStr(tg, "ID");
            t.signal       = readStr(tg, "Signal");
            t.subsystem    = readStr(tg, "Subsystem");
            t.rule_enabled = readBoolFromInt(tg, "Rule_Enabled");
            t.start_value  = readStr(tg, "Start_Value");
            t.stop_value   = readStr(tg, "Stop_Value");
            t.case_sensitivity  = readStr(tg, "Case_Sensitivity");
            t.component         = readStr(tg, "Component");
            t.cooldown_period   = readInt(tg, "Cooldown_Period");
            t.event             = readStr(tg, "Event");
            t.max_fires_per_job = readInt(tg, "Max_Fires_Per_Job");
            t.trigger_label     = readStr(tg, "Trigger_Label");
            t.extra = collectExtra(tg, {
                "ID", "Signal", "Subsystem", "Rule_Enabled", "Start_Value", "Stop_Value",
                "Case_Sensitivity", "Component", "Cooldown_Period", "Event",
                "Max_Fires_Per_Job", "Trigger_Label"
            });
            opcua.triggers[name] = std::move(t);
        }
        opcua.client = std::move(client);
        opcua.pipe   = std::move(pipe);
        return opcua;
    }

    // Return the N-th sorted Optical_Train_NN key (0-based).
    std::string trainIdAt(const HighFive::File& f, size_t train_index) const {
        auto trains_grp = f.getGroup(ROOT_OPTICAL_TRAINS);
        std::vector<std::string> ids;
        const std::string prefix = TRAIN_ID_PREFIX;
        for (const auto& n : trains_grp.listObjectNames())
            if (n.size() >= prefix.size() && n.compare(0, prefix.size(), prefix) == 0)
                ids.push_back(n);
        std::sort(ids.begin(), ids.end());
        if (train_index >= ids.size())
            throw std::out_of_range(
                "train_index " + std::to_string(train_index) +
                " out of range [0, " + std::to_string(ids.size()) + ")");
        return ids[train_index];
    }

    CorrectionData readCorrectionGrid(const HighFive::File& f, const std::string& ds_path) const {
        auto ds   = f.getDataSet(ds_path);
        auto dims = ds.getSpace().getDimensions();
        CorrectionData cd;
        cd.shape = {dims[0], dims[1], dims[2]};
        cd.data.resize(dims[0] * dims[1] * dims[2]);
        H5Dread(ds.getId(), H5T_NATIVE_DOUBLE, H5S_ALL, H5S_ALL, H5P_DEFAULT, cd.data.data());
        return cd;
    }
};

// ---------------------------------------------------------------------------
// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrateV1ToV1_1/
// migrateV1_1ToV1 from here — the coefficients/points shape-conversion
// functions they used now live in machine_config::power_characterization.hpp
// (included by the previous version's writer and this directory's own
// writer.hpp), called as a write-time fallback, not a standalone migration
// step. See that header's docs for the full design.
// ---------------------------------------------------------------------------

}  // namespace machine_config::capabilities::v1_1
