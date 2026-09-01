#pragma once
// Mock v1.1 adapter — test artifact only, exercises the change-category
// architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
// change. See the mock v1.0-to-v1.1 migration manifest under
// docs/migrations/ for the full list of differences.
//
// Design: a fully self-contained duplicate of the real v1.0 (dot-notation;
// this project's other, production File_Version) reader's/writer's
// field-by-field logic, with exactly the 10 documented differences
// implemented natively instead of as a read-then-patch pass over a
// delegated parse/write. This follows the project-wide rule that no
// version's adapter may import or call into another version's adapter
// code, even for logic that is byte-identical between versions today: if
// this mock depended on the real v1.0 adapter's code, that adapter could
// never be changed or removed later without checking this mock too, and
// every later version would compound the problem. The same fix was already
// applied to Python's real v1.1 adapter and its test-only mock; this brings
// this library's mock in line.
//
// "Unchanged" subcomponents (light_source, collimator, scanner_card,
// clearbox, sfcf, opcua) are reimplemented here field-for-field rather than
// reused, even though today they are byte-identical to the real reader's/
// writer's same-named private methods — duplication is the intended
// outcome of the isolation rule, not a smell. Mirrors the self-contained
// design of nodejs/tests/mockV1_1.ts and rust/tests/mock_v1_1/mod.rs, one
// step further than either: this library's real adapter's attribute
// helpers (readStr/readFloat/readRequiredStr/...; ws/wf/...) happen to be
// namespace-scope `inline` free functions rather than private class
// members, which used to let this mock reuse those exact primitives
// directly. They're duplicated here too now, under this file's own
// namespace, so this header has zero remaining dependency on that other
// adapter's headers or namespace.
//
// Lives in cpp/tests/ as a plain header (not test-only conditionally
// compiled) because this library is header-only with no visibility barrier
// at all — every header under include/machine_config/ is already reachable
// from any test file, unlike Rust's pub/private boundary.
//
// No dispatch-table injection, unlike Python's _ADAPTERS or Node's
// _READERS/_WRITERS: this library's public dispatcher
// (MachineConfigReader/MachineConfigWriter) is a registry keyed only by the
// real production version string, not something this mock's "1.1-mock" tag
// could be injected into without touching production code — there is no
// seam to inject a "1.1-mock" entry into. AV-09-11 therefore call this mock
// directly rather than through the public facade (see
// VALIDATION_PLAN.md §9.5).

#include "machine_config/machine_config.hpp"

#include <highfive/H5File.hpp>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstring>
#include <filesystem>
#include <iomanip>
#include <map>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

// ---------------------------------------------------------------------------
// On-disk compound-dataset row types for SynchronousSensor's two datasets.
// Duplicated (under unique names) rather than shared with the real
// adapter's equivalent types, so this header never references that
// namespace. See that adapter's own compound-types header for the full
// cross-language investigation behind the fixed 64-byte NullPadded UTF-8
// layout — every language's HDF5 binding must agree on this exact shape
// for cross-language read/write to succeed.
//
// Global scope (not inside `namespace machine_config`) because
// HIGHFIVE_REGISTER_TYPE expands to an explicit specialization of
// HighFive::create_datatype<T>, which must be declared in a namespace
// enclosing HighFive's own.
// ---------------------------------------------------------------------------

inline constexpr std::size_t MockV1_1EquationConstantMaxNameBytes = 64;

struct MockV1_1EquationConstantRow {
    char name[MockV1_1EquationConstantMaxNameBytes];
    double value;
};

inline HighFive::CompoundType create_compound_MockV1_1EquationConstantRow() {
    return {{"name", HighFive::FixedLengthStringType(
                          MockV1_1EquationConstantMaxNameBytes,
                          HighFive::StringPadding::NullPadded,
                          HighFive::CharacterSet::Utf8)},
            {"value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(MockV1_1EquationConstantRow, create_compound_MockV1_1EquationConstantRow)

struct MockV1_1CalibrationPointRow {
    double input_value;
    double output_value;
};

inline HighFive::CompoundType create_compound_MockV1_1CalibrationPointRow() {
    return {{"input_value", HighFive::create_datatype<double>()},
            {"output_value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(MockV1_1CalibrationPointRow, create_compound_MockV1_1CalibrationPointRow)

namespace machine_config::mock_v1_1 {

// ---------------------------------------------------------------------------
// On-disk layout constants
// ---------------------------------------------------------------------------

inline constexpr const char* FILE_VERSION = "1.1-mock";

inline constexpr const char* ROOT_MACHINE = "Machine";
inline constexpr const char* ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains";
inline constexpr const char* TRAIN_ID_PREFIX = "Optical_Train_";
inline constexpr const char* GROUP_OPTIONAL_COMPONENTS = "Optional_Components";
inline constexpr const char* GROUP_CLEARBOX = "ClearBox";
inline constexpr const char* DS_SCAN_FIELD_CORRECTION_FILE = "scan_field_correction_file";

inline constexpr const char* ATTR_FACILITY_ID = "Facility_ID";
inline constexpr const char* ATTR_CONFIG_AUTHOR = "Config_Author";
inline constexpr const char* ATTR_MACHINE_LABEL = "Machine_Label";
inline constexpr const char* ATTR_FOCAL_DISTANCE = "Focal_Distance";
inline constexpr const char* ATTR_BP_WIDTH = "Width";
inline constexpr const char* ATTR_BP_HEIGHT = "Height";
inline constexpr const char* SUBGROUP_DIMENSIONS = "Dimensions";

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

// ---------------------------------------------------------------------------
// Attribute-reading helpers — duplicated from the real adapter's reader so
// this header has no include-path or namespace dependency on it. All HDF5
// attribute access flows through these template functions so that they
// work uniformly for both HighFive::File and HighFive::Group.
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
            // H5T_STR_SPACEPAD; see the real reader's identical comment for the
            // full rationale. Fix: read VarLen attrs using the file's own type
            // directly.
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
        } else {
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
        } else {
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
// Attribute-writing helpers — duplicated from the real adapter's writer.
// ---------------------------------------------------------------------------

template <typename Loc>
inline void ws(Loc& loc, const std::string& key, const std::string& val) {
    loc.template createAttribute<std::string>(key, HighFive::DataSpace::Scalar()).write(val);
}

template <typename Loc>
inline void wf(Loc& loc, const std::string& key, std::optional<double> val) {
    if (val)
        loc.template createAttribute<double>(key, HighFive::DataSpace::Scalar()).write(*val);
    else
        ws(loc, key, "");
}

template <typename Loc>
inline void wi(Loc& loc, const std::string& key, std::optional<int64_t> val) {
    if (val)
        loc.template createAttribute<int64_t>(key, HighFive::DataSpace::Scalar()).write(*val);
    else
        ws(loc, key, "");
}

template <typename Loc>
inline void wb(Loc& loc, const std::string& key, std::optional<bool> val) {
    if (val) {
        int64_t v = *val ? 1LL : 0LL;
        loc.template createAttribute<int64_t>(key, HighFive::DataSpace::Scalar()).write(v);
    } else {
        ws(loc, key, "");
    }
}

template <typename Loc>
inline void wbIfTrue(Loc& loc, const std::string& key, bool val) {
    if (val) {
        loc.template createAttribute<int64_t>(key, HighFive::DataSpace::Scalar()).write(int64_t{1});
    }
}

template <typename Loc>
inline void writeExtra(Loc& loc, const ExtraAttrs& extra) {
    for (auto it = extra.begin(); it != extra.end(); ++it) {
        const std::string& k = it.key();
        const auto& v = it.value();
        if (v.is_string())
            ws(loc, k, v.template get<std::string>());
        else if (v.is_number_integer()) {
            int64_t iv = v.template get<int64_t>();
            loc.template createAttribute<int64_t>(k, HighFive::DataSpace::Scalar()).write(iv);
        } else if (v.is_number_float()) {
            double dv = v.template get<double>();
            loc.template createAttribute<double>(k, HighFive::DataSpace::Scalar()).write(dv);
        } else if (v.is_boolean()) {
            int64_t bv = v.template get<bool>() ? 1LL : 0LL;
            loc.template createAttribute<int64_t>(k, HighFive::DataSpace::Scalar()).write(bv);
        } else {
            ws(loc, k, v.dump());
        }
    }
}

// ---------------------------------------------------------------------------
// MockV1_1Reader — fully independent parser for the mock v1.1 layout.
// ---------------------------------------------------------------------------

class MockV1_1Reader {
public:
    explicit MockV1_1Reader(std::filesystem::path path) : path_(std::move(path)) {}

    MachineConfig parse() const {
        HighFive::File f(path_.string(), HighFive::File::ReadOnly);
        MachineConfig cfg;
        cfg.meta           = parseMeta(f);
        cfg.machine        = parseMachine(f.getGroup(ROOT_MACHINE));
        cfg.optical_trains = parseTrains(f);
        cfg.opcua          = parseOpcua(f);
        return cfg;
    }

private:
    std::filesystem::path path_;

    MachineConfigMeta parseMeta(const HighFive::File& f) const {
        MachineConfigMeta m;
        m.schema_version    = "v1";
        m.machine_name      = readRequiredStr(f, "machine_name");
        m.manufacturer      = readRequiredStr(f, "manufacturer");
        m.model             = readRequiredStr(f, "model");
        m.serial_number     = readRequiredStr(f, "serial_number");
        m.file_version      = readRequiredStr(f, "File_Version");
        m.export_date       = readRequiredStr(f, "Export_Date");
        m.configuration_hash= readRequiredStr(f, "Configuration_Hash");
        // ADDITION (Changes 1/2): typed root attrs with no equivalent on the
        // real adapter's version.
        m.facility_id       = readStr(f, ATTR_FACILITY_ID);
        m.config_author     = readStr(f, ATTR_CONFIG_AUTHOR);
        m.extra             = collectExtra(f, {
            "machine_name", "manufacturer", "model", "serial_number",
            "File_Version", "Export_Date", "Configuration_Hash",
            ATTR_FACILITY_ID, ATTR_CONFIG_AUTHOR
        });
        return m;
    }

    Machine parseMachine(const HighFive::Group& grp) const {
        Machine m;
        m.id                    = readStr(grp, "ID");
        // NAME (Change 5): Machine_Name -> Machine_Label.
        m.machine_name          = readRequiredStr(grp, ATTR_MACHINE_LABEL);
        m.manufacturer          = readRequiredStr(grp, "Manufacturer");
        m.model                 = readRequiredStr(grp, "Model");
        m.serial_number         = readRequiredStr(grp, "Serial_Number");
        // Unit attrs stay on Machine/ unchanged (only the numeric value
        // attrs move into Machine/Dimensions/ below).
        m.build_plate_x_unit      = readUnitLocked(grp, "Build_Plate_X_Dimension_unit", "mm");
        m.build_plate_y_unit      = readUnitLocked(grp, "Build_Plate_Y_Dimension_unit", "mm");
        m.build_plate_z_unit      = readUnitLocked(grp, "Build_Plate_Z_Dimension_unit", "mm");
        m.build_plate_radius_unit = readUnitLocked(grp, "Build_Plate_Corner_Radius_unit", "mm");
        // NAME+PATH (Changes 9/10) and PATH (Changes 7/8): numeric values
        // live under Machine/Dimensions/ now.
        if (grp.exist(SUBGROUP_DIMENSIONS)) {
            auto dims = grp.getGroup(SUBGROUP_DIMENSIONS);
            m.build_plate_x      = readFloat(dims, ATTR_BP_WIDTH);
            m.build_plate_y      = readFloat(dims, ATTR_BP_HEIGHT);
            m.build_plate_z      = readFloat(dims, "Build_Plate_Z_Dimension");
            m.build_plate_radius = readFloat(dims, "Build_Plate_Corner_Radius");
        }
        // REMOVAL (Changes 3/4): no longer read at all — always absent.
        m.gas_flow_direction = std::nullopt;
        m.recoat_direction   = std::nullopt;
        return m;
    }

    std::vector<OpticalTrain> parseTrains(const HighFive::File& f) const {
        auto trains_grp = f.getGroup(ROOT_OPTICAL_TRAINS);
        std::vector<std::string> ids;
        const std::string prefix = TRAIN_ID_PREFIX;
        for (const auto& n : trains_grp.listObjectNames()) {
            if (n.size() >= prefix.size() && n.compare(0, prefix.size(), prefix) == 0)
                ids.push_back(n);
        }
        std::sort(ids.begin(), ids.end());
        std::vector<OpticalTrain> result;
        result.reserve(ids.size());
        for (const auto& id : ids)
            result.push_back(parseTrain(f, id));
        return result;
    }

    OpticalTrain parseTrain(const HighFive::File& f, const std::string& train_id) const {
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
        // NAME (Change 6): Working_Distance -> Focal_Distance. The
        // companion _unit attribute's key does not move.
        s.working_distance      = readFloat(grp, ATTR_FOCAL_DISTANCE);
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
        sc.sample_period_unit = readUnitLocked(grp, "Sample_Period_unit", "μs");
        return sc;
    }

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
        cb.synchronous_sensors          = parseSynchronousSensors(grp);
        return cb;
    }

    static std::string fixedBufToString(const char* buf, std::size_t n) {
        std::size_t len = 0;
        while (len < n && buf[len] != '\0') ++len;
        return std::string(buf, len);
    }

    std::vector<EquationConstant> readEquationConstants(const HighFive::Group& grp) const {
        std::vector<EquationConstant> out;
        if (!grp.exist("Derivation_Equation_Constants")) return out;
        std::vector<MockV1_1EquationConstantRow> rows;
        grp.getDataSet("Derivation_Equation_Constants").read(rows);
        out.reserve(rows.size());
        for (const auto& r : rows)
            out.push_back({fixedBufToString(r.name, MockV1_1EquationConstantMaxNameBytes), r.value});
        return out;
    }

    std::vector<CalibrationPoint> readCalibrationPoints(const HighFive::Group& grp) const {
        std::vector<CalibrationPoint> out;
        if (!grp.exist("Calibration_Points")) return out;
        std::vector<MockV1_1CalibrationPointRow> rows;
        grp.getDataSet("Calibration_Points").read(rows);
        out.reserve(rows.size());
        for (const auto& r : rows)
            out.push_back({r.input_value, r.output_value});
        return out;
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
        s.derivation_equation_constants = readEquationConstants(grp);
        s.calibration_points            = readCalibrationPoints(grp);
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

        auto pipe_grp = f.getGroup("OPCUA/Pipe");
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

        auto tgrp = f.getGroup("OPCUA/Triggers");
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
};

// ---------------------------------------------------------------------------
// MockV1_1Writer — fully independent writer for the mock v1.1 layout.
// ---------------------------------------------------------------------------

class MockV1_1Writer {
public:
    explicit MockV1_1Writer(const MachineConfig& cfg) : cfg_(cfg) {}

    void write(std::filesystem::path path) const {
        HighFive::File f(path.string(),
            HighFive::File::ReadWrite | HighFive::File::Create | HighFive::File::Truncate);
        writeRootAttrs(f);
        auto mgrp = f.createGroup(ROOT_MACHINE);
        writeMachineAttrs(mgrp);
        auto ogrp = mgrp.createGroup("Optical_Trains");
        for (size_t i = 0; i < cfg_.optical_trains.size(); ++i) {
            auto tg = ogrp.createGroup(trainId(i));
            writeTrain(tg, cfg_.optical_trains[i]);
        }
        if (cfg_.opcua) writeOpcua(f, *cfg_.opcua);
    }

private:
    const MachineConfig& cfg_;

    void writeRootAttrs(HighFive::File& f) const {
        const auto& m = cfg_.meta;
        ws(f, "machine_name",       m.machine_name);
        ws(f, "manufacturer",       m.manufacturer);
        ws(f, "model",              m.model);
        ws(f, "serial_number",      m.serial_number);
        ws(f, "File_Version",       m.file_version);
        ws(f, "Export_Date",        m.export_date);
        ws(f, "Configuration_Hash", m.configuration_hash);
        // ADDITION (Changes 1/2).
        ws(f, ATTR_FACILITY_ID,     m.facility_id.value_or(""));
        ws(f, ATTR_CONFIG_AUTHOR,   m.config_author.value_or(""));
        writeExtra(f, m.extra);
    }

    void writeMachineAttrs(HighFive::Group& grp) const {
        const auto& ma = cfg_.machine;
        ws(grp, "ID",                            ma.id.value_or(""));
        // NAME (Change 5).
        ws(grp, ATTR_MACHINE_LABEL,              ma.machine_name);
        ws(grp, "Manufacturer",                  ma.manufacturer);
        ws(grp, "Model",                         ma.model);
        ws(grp, "Serial_Number",                 ma.serial_number);
        // Unit attrs stay on Machine/ unchanged.
        ws(grp, "Build_Plate_X_Dimension_unit",  ma.build_plate_x_unit.value_or("mm"));
        ws(grp, "Build_Plate_Y_Dimension_unit",  ma.build_plate_y_unit.value_or("mm"));
        ws(grp, "Build_Plate_Z_Dimension_unit",  ma.build_plate_z_unit.value_or("mm"));
        ws(grp, "Build_Plate_Corner_Radius_unit",ma.build_plate_radius_unit.value_or("mm"));
        // REMOVAL (Changes 3/4): Gas_Flow_Direction / Recoat_Direction are
        // never written at all.

        // PATH (Changes 7/8) and NAME+PATH (Changes 9/10): numeric values
        // move into Machine/Dimensions/.
        auto dims = grp.createGroup(SUBGROUP_DIMENSIONS);
        wf(dims, ATTR_BP_WIDTH,               ma.build_plate_x);
        wf(dims, ATTR_BP_HEIGHT,              ma.build_plate_y);
        wf(dims, "Build_Plate_Z_Dimension",   ma.build_plate_z);
        wf(dims, "Build_Plate_Corner_Radius", ma.build_plate_radius);
    }

    void writeTrain(HighFive::Group& grp, const OpticalTrain& t) const {
        ws(grp, "ID",                                 t.id.value_or(""));
        ws(grp, "Beam_Profile_Type",                  t.beam_profile_type.value_or(""));
        ws(grp, "Beam_Waist_Definition",              t.beam_waist_definition.value_or(""));
        wf(grp, "Beam_Waist_Major",                   t.beam_waist_major);
        ws(grp, "Beam_Waist_Major_unit",              t.beam_waist_major_unit.value_or("\xce\xbcm"));
        wf(grp, "Beam_Waist_Minor",                   t.beam_waist_minor);
        ws(grp, "Beam_Waist_Minor_unit",              t.beam_waist_minor_unit.value_or("\xce\xbcm"));
        wf(grp, "Beam_Waist_Offset_Z",                t.beam_waist_offset_z);
        ws(grp, "Beam_Waist_Offset_Z_unit",           t.beam_waist_offset_z_unit.value_or("mm"));
        wf(grp, "Build_Plane_Offset_Major",           t.build_plane_offset_major);
        ws(grp, "Build_Plane_Offset_Major_unit",      t.build_plane_offset_major_unit.value_or("mm"));
        wf(grp, "Build_Plane_Offset_Minor",           t.build_plane_offset_minor);
        ws(grp, "Build_Plane_Offset_Minor_unit",      t.build_plane_offset_minor_unit.value_or("mm"));
        wf(grp, "Collimator_Focal_Length",            t.collimator_focal_length);
        ws(grp, "Collimator_Focal_Length_unit",       t.collimator_focal_length_unit.value_or("mm"));
        wf(grp, "M2_Major",                           t.m2_major);
        wf(grp, "M2_Minor",                           t.m2_minor);
        wf(grp, "Major_Axis_Angle",                   t.major_axis_angle);
        ws(grp, "Major_Axis_Angle_unit",              t.major_axis_angle_unit.value_or("degrees"));
        wf(grp, "Rayleigh_Length_Major",              t.rayleigh_length_major);
        ws(grp, "Rayleigh_Length_Major_unit",         t.rayleigh_length_major_unit.value_or("mm"));
        wf(grp, "Rayleigh_Length_Minor",              t.rayleigh_length_minor);
        ws(grp, "Rayleigh_Length_Minor_unit",         t.rayleigh_length_minor_unit.value_or("mm"));
        ws(grp, "Scanner_Number",                     t.scanner_number.value_or(""));
        wb(grp, "Thermal_Lensing_Test_Passed",        t.thermal_lensing_passed);
        wf(grp, "Thermal_Lensing_Focal_Plane_Shift",       t.thermal_lensing_focal_plane_shift);
        ws(grp, "Thermal_Lensing_Focal_Plane_Shift_unit",  t.thermal_lensing_focal_plane_shift_unit.value_or("mm"));
        wf(grp, "Thermal_Lensing_Threshold",          t.thermal_lensing_threshold);
        ws(grp, "Thermal_Lensing_Threshold_unit",     t.thermal_lensing_threshold_unit.value_or("mm"));

        auto sg = grp.createGroup("Scanner");      writeScanner(sg, t.scanner);
        auto lg = grp.createGroup("Light_Source"); writeLightSource(lg, t.light_source);
        auto cg = grp.createGroup("Collimator");   writeCollimator(cg, t.collimator);
        auto kg = grp.createGroup("Scanner_Card"); writeScannerCard(kg, t.scanner_card);

        if (t.optional_components.clearbox) {
            auto og = grp.createGroup("Optional_Components");
            auto bg = og.createGroup("ClearBox");
            writeClearBox(bg, *t.optional_components.clearbox);
        }
        if (t.scan_field_correction_file)
            writeSfcf(grp, *t.scan_field_correction_file);
    }

    void writeAxis(HighFive::Group& grp, const AxisConfig& ax) const {
        wi(grp, "Actual_Bit_Resolution",        ax.actual_bit_resolution);
        ws(grp, "Actual_Bit_Resolution_unit",   ax.actual_bit_resolution_unit.value_or(""));
        wi(grp, "Commanded_Bit_Resolution",     ax.commanded_bit_resolution);
        ws(grp, "Commanded_Bit_Resolution_unit",ax.commanded_bit_resolution_unit.value_or(""));
        ws(grp, "Control_Type",                 ax.control_type.value_or(""));
        wf(grp, "Range_Of_Motion",              ax.range_of_motion);
        ws(grp, "Range_Of_Motion_unit",         ax.range_of_motion_unit.value_or(""));
        ws(grp, "Smoothing_Kernel",             ax.smoothing_kernel.value_or(""));
        wf(grp, "Smoothing_Parameters",         ax.smoothing_parameters);
        ws(grp, "Tuning_Parameters",            ax.tuning_parameters.value_or(""));
        ws(grp, "Tuning_Type",                  ax.tuning_type.value_or(""));
    }

    void writeScanner(HighFive::Group& grp, const Scanner& s) const {
        ws(grp, "Manufacturer",            s.manufacturer);
        ws(grp, "Model",                   s.model);
        ws(grp, "Serial_Number",           s.serial_number);
        // NAME (Change 6): value moves to Focal_Distance; the unit key stays
        // Working_Distance_unit.
        wf(grp, ATTR_FOCAL_DISTANCE,       s.working_distance);
        ws(grp, "Working_Distance_unit",   s.working_distance_unit.value_or("mm"));
        wf(grp, "Scan_Field_Size_X",       s.scan_field_x);
        ws(grp, "Scan_Field_Size_X_unit",  s.scan_field_x_unit.value_or("mm"));
        wf(grp, "Scan_Field_Size_Y",       s.scan_field_y);
        ws(grp, "Scan_Field_Size_Y_unit",  s.scan_field_y_unit.value_or("mm"));
        wf(grp, "Scan_Field_Size_Z",       s.scan_field_z);
        ws(grp, "Scan_Field_Size_Z_unit",  s.scan_field_z_unit.value_or("mm"));
        wf(grp, "Scan_Head_Offset_X",      s.scan_head_offset_x);
        ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit.value_or("mm"));
        wf(grp, "Scan_Head_Offset_Y",      s.scan_head_offset_y);
        ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit.value_or("mm"));
        wf(grp, "Scan_Head_Offset_Z",      s.scan_head_offset_z);
        ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit.value_or("mm"));
        wf(grp, "Scan_Head_Rotation",      s.scan_head_rotation);
        ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit.value_or("degrees"));
        ws(grp, "Axis_Configuration",      s.axis_configuration.value_or(""));
        wbIfTrue(grp, "Invert_Actual_X",      s.invert_actual_x);
        wbIfTrue(grp, "Invert_Actual_Y",      s.invert_actual_y);
        wbIfTrue(grp, "Invert_Commanded_X",   s.invert_commanded_x);
        wbIfTrue(grp, "Invert_Commanded_Y",   s.invert_commanded_y);
        auto xg = grp.createGroup("X_Axis"); writeAxis(xg, s.x_axis);
        auto yg = grp.createGroup("Y_Axis"); writeAxis(yg, s.y_axis);
        if (s.z_axis) { auto zg = grp.createGroup("Z_Axis"); writeAxis(zg, *s.z_axis); }
        if (s.focus)  { auto fg = grp.createGroup("Focus");  writeAxis(fg, *s.focus);  }
    }

    void writeLightSource(HighFive::Group& grp, const LightSource& ls) const {
        ws(grp, "Manufacturer",               ls.manufacturer);
        ws(grp, "Model",                      ls.model);
        ws(grp, "Serial_Number",              ls.serial_number);
        wf(grp, "Light_Wavelength",           ls.wavelength);
        ws(grp, "Light_Wavelength_unit",      ls.wavelength_unit.value_or("nm"));
        wf(grp, "Power_Max_Nominal",          ls.power_max_nominal);
        ws(grp, "Power_Max_Nominal_unit",     ls.power_max_nominal_unit.value_or("W"));
        wf(grp, "Power_Max_Actual",           ls.power_max_actual);
        ws(grp, "Power_Max_Actual_unit",      ls.power_max_actual_unit.value_or("W"));
        wf(grp, "Power_Min_Actual",           ls.power_min_actual);
        ws(grp, "Power_Min_Actual_unit",      ls.power_min_actual_unit.value_or("W"));
        wf(grp, "Power_Min_Nominal",          ls.power_min_nominal);
        ws(grp, "Power_Min_Nominal_unit",     ls.power_min_nominal_unit.value_or("W"));
        // Real HDF5 files store Power_Bit_Resolution as a string.
        std::string pbr = ls.power_bit_resolution
            ? nlohmann::json(*ls.power_bit_resolution).dump()
            : std::string{};
        ws(grp, "Power_Bit_Resolution",       pbr);
        ws(grp, "Power_Bit_Resolution_unit",  ls.power_bit_resolution_unit.value_or("bits"));
        ws(grp, "Watts_To_Volts_Algorithm",   ls.watts_to_volts_algorithm.value_or(""));
        ws(grp, "Watts_To_Volts_Params",      ls.watts_to_volts_params.value_or(""));
    }

    void writeCollimator(HighFive::Group& grp, const Collimator& c) const {
        ws(grp, "Manufacturer",      c.manufacturer);
        ws(grp, "Model",             c.model);
        ws(grp, "Serial_Number",     c.serial_number);
        wf(grp, "Focal_Length",      c.focal_length);
        ws(grp, "Focal_Length_unit", c.focal_length_unit.value_or("mm"));
    }

    void writeScannerCard(HighFive::Group& grp, const ScannerCard& sc) const {
        ws(grp, "Manufacturer",            sc.manufacturer);
        ws(grp, "Model",                   sc.model);
        ws(grp, "Serial_Number",           sc.serial_number);
        ws(grp, "Communication_Protocol",  sc.communication_protocol.value_or(""));
        wf(grp, "Sample_Period",           sc.sample_period);
        ws(grp, "Sample_Period_unit",      sc.sample_period_unit.value_or("\xce\xbcs"));
    }

    void writeClearBox(HighFive::Group& grp, const ClearBox& cb) const {
        ws(grp, "Ip_Address",                  cb.ip_address);
        ws(grp, "Serial_Number",               cb.serial_number.value_or(""));
        wi(grp, "Data_Port",                   cb.data_port);
        wi(grp, "Server_Port",                 cb.server_port);
        wi(grp, "Actual_Timing_Offset",        cb.actual_timing_offset);
        wi(grp, "Commanded_Timing_Offset",     cb.commanded_timing_offset);
        ws(grp, "Manufacturer",                cb.manufacturer.value_or(""));
        ws(grp, "Model",                       cb.model.value_or(""));
        ws(grp, "Output_Path",                 cb.output_path.value_or(""));
        ws(grp, "Selected_Camera",             cb.selected_camera.value_or(""));
        ws(grp, "Custom_Video_Format",         cb.custom_video_format.value_or(""));
        ws(grp, "Video_Output",                cb.video_output.value_or(""));
        wb(grp, "Show_Console",                cb.show_console);
        wi(grp, "Software_Trigger_Delay",      cb.software_trigger_delay);
        ws(grp, "Volts_To_Watts_Algorithm",    cb.volts_to_watts_algorithm.value_or(""));
        ws(grp, "Volts_To_Watts_Params",       cb.volts_to_watts_params.value_or(""));
        ws(grp, "Correction_Grid_Domain_Shape",cb.correction_grid_domain_shape.value_or(""));
        ws(grp, "Inverse_Grid_Domain_Shape",   cb.inverse_grid_domain_shape.value_or(""));
        writeCorrectionDataset(grp, "Correction_Data",         cb.correction_data);
        writeCorrectionDataset(grp, "Inverse_Correction_Data", cb.inverse_correction_data);

        if (!cb.synchronous_sensors.empty()) {
            auto sensors_grp = grp.createGroup("Synchronous_Sensors");
            for (const auto& [name, sensor] : cb.synchronous_sensors) {
                auto sg = sensors_grp.createGroup(name);
                writeSynchronousSensor(sg, sensor);
            }
        }
    }

    void writeEquationConstants(HighFive::Group& grp,
                                 const std::vector<EquationConstant>& constants) const {
        std::vector<MockV1_1EquationConstantRow> rows;
        rows.reserve(constants.size());
        for (const auto& c : constants) {
            if (c.name.size() > MockV1_1EquationConstantMaxNameBytes) {
                throw std::runtime_error(
                    "Derivation_Equation_Constants name '" + c.name + "' is " +
                    std::to_string(c.name.size()) + " UTF-8 bytes, which does not fit in the " +
                    std::to_string(MockV1_1EquationConstantMaxNameBytes) +
                    "-byte fixed-length field (would otherwise be silently truncated on write).");
            }
            MockV1_1EquationConstantRow row{};
            std::memset(row.name, 0, sizeof(row.name));
            std::memcpy(row.name, c.name.data(), c.name.size());
            row.value = c.value;
            rows.push_back(row);
        }
        grp.createDataSet("Derivation_Equation_Constants", rows);
    }

    void writeCalibrationPoints(HighFive::Group& grp,
                                 const std::vector<CalibrationPoint>& points) const {
        std::vector<MockV1_1CalibrationPointRow> rows;
        rows.reserve(points.size());
        for (const auto& p : points)
            rows.push_back({p.input_value, p.output_value});
        grp.createDataSet("Calibration_Points", rows);
    }

    void writeSynchronousSensor(HighFive::Group& grp, const SynchronousSensor& s) const {
        wb(grp, "Enabled", s.enabled);
        ws(grp, "Sensor_Name", s.sensor_name.value_or(""));
        wf(grp, "Sensor_Output_Range_Low", s.sensor_output_range_low);
        wf(grp, "Sensor_Output_Range_High", s.sensor_output_range_high);
        ws(grp, "Sensor_Output_Space", s.sensor_output_space.value_or(""));
        ws(grp, "Sensor_Model", s.sensor_model.value_or(""));
        ws(grp, "Sensor_Manufacturer", s.sensor_manufacturer.value_or(""));
        ws(grp, "Sensor_Scope", s.sensor_scope.value_or(""));
        ws(grp, "Units_Derived_Quantity", s.units_derived_quantity.value_or(""));
        wi(grp, "Port_ID", s.port_id);
        ws(grp, "Sensor_Type", s.sensor_type.value_or(""));
        ws(grp, "Input_Type", s.input_type.value_or(""));
        ws(grp, "Algorithm_Type", s.algorithm_type.value_or(""));
        ws(grp, "Algorithm_Equation", s.algorithm_equation.value_or(""));
        ws(grp, "Calibration_Source", s.calibration_source.value_or(""));
        wb(grp, "Calibration_Verified", s.calibration_verified);
        wf(grp, "Sample_Period", s.sample_period);
        ws(grp, "Metadata", s.metadata.value_or(""));
        writeEquationConstants(grp, s.derivation_equation_constants);
        writeCalibrationPoints(grp, s.calibration_points);
    }

    void writeCorrectionDataset(HighFive::Group& grp, const std::string& name,
                                  const std::optional<Grid3D>& grid) const {
        std::array<size_t, 3> shape{};
        auto flat = detail::gridToFlat(grid, shape);
        auto ds = grp.createDataSet<double>(
            name, HighFive::DataSpace({shape[0], shape[1], shape[2]}));
        H5Dwrite(ds.getId(), H5T_NATIVE_DOUBLE, H5S_ALL, H5S_ALL, H5P_DEFAULT, flat.data());
        ws(ds, "dimensions", "H,W,D");
        ws(ds, "dtype", "float64");
        ws(ds, "shape", std::to_string(shape[0]) + "x" +
                        std::to_string(shape[1]) + "x" +
                        std::to_string(shape[2]));
    }

    void writeSfcf(HighFive::Group& train_grp, const ScanFieldCorrectionFile& sfcf) const {
        std::vector<uint8_t> data;
        if (sfcf.raw_bytes)
            data = *sfcf.raw_bytes;
        else
            data.resize(static_cast<size_t>(sfcf.file_size > 0 ? sfcf.file_size : 1), 0);
        auto ds = train_grp.createDataSet<uint8_t>(
            DS_SCAN_FIELD_CORRECTION_FILE, HighFive::DataSpace({data.size()}));
        H5Dwrite(ds.getId(), H5T_NATIVE_UINT8, H5S_ALL, H5S_ALL, H5P_DEFAULT, data.data());
        ws(ds, "document_name",       sfcf.document_name);
        ws(ds, "document_id",         sfcf.document_id);
        ds.template createAttribute<int64_t>(
            "file_size", HighFive::DataSpace::Scalar()).write(sfcf.file_size);
        ws(ds, "valid_as_of_date",    sfcf.valid_as_of_date);
        ws(ds, "document_created_at", sfcf.document_created_at.value_or(""));
        ws(ds, "document_type",       sfcf.document_type.value_or(""));
        ws(ds, "original_uri",        sfcf.original_uri.value_or(""));
    }

    void writeOpcua(HighFive::File& f, const OpcuaConfig& opcua) const {
        auto og = f.createGroup("OPCUA");

        auto cg = og.createGroup("Client");
        const auto& c = opcua.client;
        ws(cg, "Server_URL",      c.server_url);
        ws(cg, "Auth_Mode",       c.auth_mode);
        ws(cg, "Security_Mode",   c.security_mode);
        ws(cg, "Security_Policy", c.security_policy);
        cg.template createAttribute<int64_t>("BFS_Max_Depth",     HighFive::DataSpace::Scalar()).write(c.bfs_max_depth);
        cg.template createAttribute<int64_t>("Publish_Interval",  HighFive::DataSpace::Scalar()).write(c.publish_interval);
        cg.template createAttribute<int64_t>("Sampling_Interval", HighFive::DataSpace::Scalar()).write(c.sampling_interval);
        cg.template createAttribute<int64_t>("Session_Timeout",   HighFive::DataSpace::Scalar()).write(c.session_timeout);
        wi(cg, "Keep_Alive_Count", c.keep_alive_count);
        wi(cg, "Lifetime_Count", c.lifetime_count);
        ws(cg, "Machine_Profile", c.machine_profile.value_or(""));
        ws(cg, "Queue_Policy", c.queue_policy.value_or(""));
        wi(cg, "Queue_Size_Data_Change", c.queue_size_data_change);
        wi(cg, "Queue_Size_Events", c.queue_size_events);
        wi(cg, "Reconnect_Interval", c.reconnect_interval);
        ws(cg, "Root_Node", c.root_node.value_or(""));
        wi(cg, "Sync_Loop_Interval_Initial", c.sync_loop_interval_initial);
        wi(cg, "Sync_Loop_Interval_Settled", c.sync_loop_interval_settled);
        writeExtra(cg, c.extra);

        auto pg = og.createGroup("Pipe");
        const auto& p = opcua.pipe;
        pg.template createAttribute<int64_t>("Pipe_Enabled", HighFive::DataSpace::Scalar()).write(static_cast<int64_t>(p.pipe_enabled));
        pg.template createAttribute<int64_t>("Buffer_Size",  HighFive::DataSpace::Scalar()).write(p.buffer_size);
        wb(pg, "Configure_Client", p.configure_client);
        wi(pg, "Inbound_Rate_Limit", p.inbound_rate_limit);
        wi(pg, "Max_Inbound_Message_Size", p.max_inbound_message_size);
        ws(pg, "Min_Integrity_Level", p.min_integrity_level.value_or(""));
        ws(pg, "Pipe_Name", p.pipe_name.value_or(""));
        ws(pg, "User_Access_Level", p.user_access_level.value_or(""));
        writeExtra(pg, p.extra);

        auto tg = og.createGroup("Triggers");
        if (opcua.triggers_enabled) {
            // Stored as float64 to match original HDF5 file format.
            double te = *opcua.triggers_enabled ? 1.0 : 0.0;
            tg.template createAttribute<double>("Triggers_Enabled", HighFive::DataSpace::Scalar()).write(te);
        }
        wi(tg, "Trigger_Stop_Ceiling_Layers", opcua.trigger_stop_ceiling_layers);
        for (const auto& [name, trigger] : opcua.triggers) {
            auto trg = tg.createGroup(name);
            ws(trg, "ID",          trigger.id.value_or(""));
            ws(trg, "Signal",      trigger.signal.value_or(""));
            ws(trg, "Subsystem",   trigger.subsystem.value_or(""));
            wb(trg, "Rule_Enabled",trigger.rule_enabled);
            ws(trg, "Start_Value", trigger.start_value.value_or(""));
            ws(trg, "Stop_Value",  trigger.stop_value.value_or(""));
            ws(trg, "Case_Sensitivity", trigger.case_sensitivity.value_or(""));
            ws(trg, "Component", trigger.component.value_or(""));
            wi(trg, "Cooldown_Period", trigger.cooldown_period);
            ws(trg, "Event", trigger.event.value_or(""));
            wi(trg, "Max_Fires_Per_Job", trigger.max_fires_per_job);
            ws(trg, "Trigger_Label", trigger.trigger_label.value_or(""));
            writeExtra(trg, trigger.extra);
        }
    }
};

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's _make_config() / Node's makeMockConfig
// ---------------------------------------------------------------------------

inline MachineConfig makeMockConfig(
    const std::string& machineName = "MigrationTestMachine",
    std::optional<std::string> facilityId = std::nullopt,
    std::optional<std::string> configAuthor = std::nullopt)
{
    MockConfigBuilder builder;
    builder.laser_count = 1;
    builder.build_plate_x = 250.0;
    builder.build_plate_y = 175.0; // distinct from x so name+path assertions are unambiguous
    builder.machine_name = machineName;

    MachineConfig cfg = builder.build();
    cfg.meta.file_version = FILE_VERSION;
    cfg.meta.facility_id = std::move(facilityId);
    cfg.meta.config_author = std::move(configAuthor);
    cfg.machine.gas_flow_direction = std::nullopt; // absent in v1.1-mock by design
    cfg.machine.recoat_direction = std::nullopt;
    // MockConfigBuilder never sets this — give it a real value so the PATH
    // category (Build_Plate_Corner_Radius) has something non-trivial to
    // verify preservation of, mirroring Rust's mock_v1_1 factory.
    cfg.machine.build_plate_radius = 10.0;
    return cfg;
}

} // namespace machine_config::mock_v1_1
