#pragma once
// Machine Config Library — C++ data models.
// Mirrors python/src/machine_config/models.py and rust/src/models.rs.
// Type mapping: §4.6 of IMPLEMENTATION_PLAN.md.
//
// JSON field names and structure are authoritative from Python _config_to_dict().
// Key layout decisions:
//   - Machine is FLAT (build_plate fields inlined, not nested); canonical field
//     name is "build_plate_radius" (not "build_plate_corner_radius").
//   - correction_data / inverse_correction_data / raw_bytes / opcua are OMITTED
//     (not serialised as null) when absent.  All other optional fields → JSON null.
//   - Grid cells use std::optional<double>: nullopt = HDF5 NaN / out-of-field.
//   - CorrectionData (flat buffer) is for binary ops only; not serialised to JSON.

#include <nlohmann/json.hpp>

#include <array>
#include <cstddef>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace machine_config {

// ---------------------------------------------------------------------------
// Internal serialisation helpers
// ---------------------------------------------------------------------------

namespace detail {

// Serialise std::optional<T> as JSON null when empty, value otherwise.
template <typename T>
inline nlohmann::json opt_to_j(const std::optional<T>& v) {
    return v ? nlohmann::json(*v) : nlohmann::json(nullptr);
}

// Read an optional field: returns nullopt when the key is missing or null.
template <typename T>
inline std::optional<T> j_to_opt(const nlohmann::json& j, const char* key) {
    auto it = j.find(key);
    if (it == j.end() || it->is_null()) return std::nullopt;
    return it->get<T>();
}

} // namespace detail

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

// Arbitrary non-schema HDF5 attributes preserved verbatim.
using ExtraAttrs = nlohmann::json;

// JSON representation of a correction grid cell.
// nullopt = out-of-field position (HDF5 NaN); double = correction value.
using GridCell = std::optional<double>;
using Grid3D   = std::vector<std::vector<std::vector<GridCell>>>;

// ---------------------------------------------------------------------------
// CorrectionData — flat buffer for binary operations (correction-hash, copy-hdf5).
// Not serialised to JSON directly; produced by MachineConfigReader::getCorrectionData().
// ---------------------------------------------------------------------------

struct CorrectionData {
    // Flat row-major buffer.  Element at (i,j,k): data[i*shape[1]*shape[2] + j*shape[2] + k].
    // NaN values represent out-of-field cells.
    std::vector<double> data;
    std::array<std::size_t, 3> shape; // {d0, d1, d2}, e.g. {257, 257, 2}
};

// ---------------------------------------------------------------------------
// Structs — defined bottom-up (inner before outer)
// ---------------------------------------------------------------------------

// Metadata for the embedded .fc3 scan-field correction file.
// HDF5 source: a Dataset (not a Group) at .../scan_field_correction_file.
struct ScanFieldCorrectionFile {
    std::string document_name;
    std::string document_id;
    std::int64_t file_size{0};
    std::string valid_as_of_date;
    std::optional<std::string> document_created_at;
    std::optional<std::string> document_type;
    std::optional<std::string> original_uri;
    // raw_bytes: populated only when include_binary=true; base64-encoded in JSON.
    // Omitted from JSON entirely when absent.
    std::optional<std::vector<std::uint8_t>> raw_bytes;
};

// One named constant used to derive an equation (e.g. `a`/`b` for a
// Log-Linear fit, `c0`..`cN` for a polynomial fit). Stored on disk as a
// 64-byte fixed-length UTF-8 string (see SYNCHRONOUS_SENSOR_PLAN.md's
// "Compound dataset string convention") — name here is a plain
// std::string; the fixed-width conversion happens only at the HDF5 adapter
// layer (capabilities::v1_0's EquationConstantRow).
struct EquationConstant {
    std::string name;
    double value{0.0};
};

// One raw calibration pair. input_value is in whatever unit
// SynchronousSensor::input_type implies; output_value is in whatever unit
// SynchronousSensor::sensor_output_space implies (no per-row unit tag) —
// see SYNCHRONOUS_SENSOR_PLAN.md's "Why compound datasets" for the
// convention.
struct CalibrationPoint {
    double input_value{0.0};
    double output_value{0.0};
};

// One Synchronous Sensor record. The map key (on ClearBox::synchronous_sensors)
// is a free-form label chosen by the file's author — not required to equal
// any attribute value inside the sensor's own group (same convention as
// OpcuaConfig::triggers's keys).
struct SynchronousSensor {
    std::optional<bool>        enabled;
    std::optional<std::string> sensor_name;
    std::optional<double>      sensor_output_range_low;
    std::optional<double>      sensor_output_range_high;
    std::optional<std::string> sensor_output_space;
    std::optional<std::string> sensor_model;
    std::optional<std::string> sensor_manufacturer;
    std::optional<std::string> sensor_scope;
    std::optional<std::string> units_derived_quantity;
    std::optional<std::int64_t> port_id;
    std::optional<std::string> sensor_type;
    std::optional<std::string> input_type;
    std::optional<std::string> algorithm_type;
    std::optional<std::string> algorithm_equation;
    std::optional<std::string> calibration_source;
    std::optional<bool>        calibration_verified;
    std::optional<double>      sample_period;
    std::optional<std::string> metadata;
    std::vector<EquationConstant> derivation_equation_constants;
    std::vector<CalibrationPoint> calibration_points;
};

// Optional ClearBox add-on.  HDF5: .../Optional_Components/ClearBox/.
struct ClearBox {
    std::string ip_address;
    std::optional<std::string>  serial_number;
    std::optional<std::int64_t> data_port;
    std::optional<std::int64_t> server_port;
    std::optional<std::int64_t> actual_timing_offset;
    std::optional<std::int64_t> commanded_timing_offset;
    // correction_data / inverse_correction_data: omitted from JSON when absent
    // (include_binary=false).  Populated by the reader when include_binary=true.
    std::optional<Grid3D> correction_data;
    std::optional<Grid3D> inverse_correction_data;
    std::optional<std::string>  manufacturer;
    std::optional<std::string>  model;
    std::optional<std::string>  output_path;
    std::optional<std::string>  selected_camera;
    std::optional<std::string>  custom_video_format;
    std::optional<std::string>  video_output;
    std::optional<bool>         show_console; // HDF5 int 0/1
    std::optional<std::int64_t> software_trigger_delay;
    std::optional<std::string>  volts_to_watts_algorithm;
    std::optional<std::string>  volts_to_watts_params;
    std::optional<std::string>  correction_grid_domain_shape;
    std::optional<std::string>  inverse_grid_domain_shape;
    // Omitted entirely (not "{}") when there are no sensors — matches
    // Rust's skip_serializing_if and Python's _clearbox_to_dict choice to
    // omit the key when empty, so every fixture that doesn't use this
    // feature stays byte-for-byte identical in JSON shape to before it
    // existed. Same optional-whole-feature shape as MachineConfig::opcua,
    // not OpcuaConfig::triggers (which is always present, even as "{}") —
    // deliberately different from that precedent (see the Node.js bug this
    // exact mismatch caused, corrected before this language's Phase 1).
    std::map<std::string, SynchronousSensor> synchronous_sensors;
};

// Optional add-on hardware present on an optical train.
struct OptionalComponents {
    std::optional<ClearBox> clearbox;
};

struct Collimator {
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::optional<double>      focal_length;
    std::optional<std::string> focal_length_unit;
};

struct ScannerCard {
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::optional<std::string> communication_protocol;
    std::optional<double>      sample_period;
    std::optional<std::string> sample_period_unit;
};

// One scanner axis tuning sub-group (X_Axis, Y_Axis, Z_Axis, Focus).
struct AxisConfig {
    std::optional<std::int64_t> actual_bit_resolution;
    std::optional<std::string>  actual_bit_resolution_unit;
    std::optional<std::int64_t> commanded_bit_resolution;
    std::optional<std::string>  commanded_bit_resolution_unit;
    std::optional<std::string>  control_type;
    std::optional<double>       range_of_motion;
    std::optional<std::string>  range_of_motion_unit;
    std::optional<std::string>  smoothing_kernel;
    std::optional<double>       smoothing_parameters;
    std::optional<std::string>  tuning_parameters;
    std::optional<std::string>  tuning_type;
};

struct Scanner {
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::optional<double>      working_distance;
    std::optional<std::string> working_distance_unit;
    std::optional<double>      scan_field_x;
    std::optional<std::string> scan_field_x_unit;
    std::optional<double>      scan_field_y;
    std::optional<std::string> scan_field_y_unit;
    std::optional<double>      scan_field_z;
    std::optional<std::string> scan_field_z_unit;
    std::optional<double>      scan_head_offset_x;
    std::optional<std::string> scan_head_offset_x_unit;
    std::optional<double>      scan_head_offset_y;
    std::optional<std::string> scan_head_offset_y_unit;
    std::optional<double>      scan_head_offset_z;
    std::optional<std::string> scan_head_offset_z_unit;
    std::optional<double>      scan_head_rotation;
    std::optional<std::string> scan_head_rotation_unit;
    // "2D" / "3D" / "3D+Focus"; constraint enforced by reader, not here.
    std::optional<std::string> axis_configuration;
    AxisConfig x_axis;
    AxisConfig y_axis;
    std::optional<AxisConfig> z_axis;
    std::optional<AxisConfig> focus;
};

struct LightSource {
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::optional<double>      wavelength;
    std::optional<std::string> wavelength_unit;
    std::optional<double>      power_max_nominal;
    std::optional<std::string> power_max_nominal_unit;
    std::optional<double>      power_max_actual;
    std::optional<std::string> power_max_actual_unit;
    std::optional<double>      power_min_actual;
    std::optional<std::string> power_min_actual_unit;
    std::optional<double>      power_min_nominal;
    std::optional<std::string> power_min_nominal_unit;
    // HDF5 stores this attribute as a string; the reader parses it to double.
    std::optional<double>      power_bit_resolution;
    std::optional<std::string> power_bit_resolution_unit;
    std::optional<std::string> watts_to_volts_algorithm;
    std::optional<std::string> watts_to_volts_params;
};

struct OpticalTrain {
    std::string train_id;
    std::optional<std::string> id; // UUID on the train group, often empty
    std::optional<std::string> beam_profile_type;
    std::optional<std::string> beam_waist_definition;
    std::optional<double>      beam_waist_major;
    std::optional<std::string> beam_waist_major_unit;
    std::optional<double>      beam_waist_minor;
    std::optional<std::string> beam_waist_minor_unit;
    std::optional<double>      beam_waist_offset_z;
    std::optional<std::string> beam_waist_offset_z_unit;
    std::optional<double>      build_plane_offset_major;
    std::optional<std::string> build_plane_offset_major_unit;
    std::optional<double>      build_plane_offset_minor;
    std::optional<std::string> build_plane_offset_minor_unit;
    // Train-level duplicate of collimator.focal_length.
    std::optional<double>      collimator_focal_length;
    std::optional<std::string> collimator_focal_length_unit;
    std::optional<double>      m2_major;
    std::optional<double>      m2_minor;
    std::optional<double>      major_axis_angle;
    std::optional<std::string> major_axis_angle_unit;
    std::optional<double>      rayleigh_length_major;
    std::optional<std::string> rayleigh_length_major_unit;
    std::optional<double>      rayleigh_length_minor;
    std::optional<std::string> rayleigh_length_minor_unit;
    std::optional<std::string> scanner_number;
    std::optional<bool>        thermal_lensing_passed; // HDF5 int 0/1
    std::optional<double>      thermal_lensing_focal_plane_shift;
    std::optional<std::string> thermal_lensing_focal_plane_shift_unit;
    std::optional<double>      thermal_lensing_threshold;
    std::optional<std::string> thermal_lensing_threshold_unit;
    Scanner            scanner;
    LightSource        light_source;
    Collimator         collimator;
    ScannerCard        scanner_card;
    OptionalComponents optional_components;
    std::optional<ScanFieldCorrectionFile> scan_field_correction_file;
};

// Machine-level attributes.  Build-plate fields are INLINED here (not nested)
// to match the flat "machine" object in the canonical JSON schema (§0.2, §3.10).
struct Machine {
    std::optional<std::string> id;
    std::string machine_name;
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::optional<double>      build_plate_x;
    std::optional<std::string> build_plate_x_unit;
    std::optional<double>      build_plate_y;
    std::optional<std::string> build_plate_y_unit;
    std::optional<double>      build_plate_z;
    std::optional<std::string> build_plate_z_unit;
    std::optional<double>      build_plate_radius;
    std::optional<std::string> build_plate_radius_unit;
    std::optional<std::string> gas_flow_direction;
    std::optional<std::string> recoat_direction;
};

struct MachineConfigMeta {
    std::string schema_version;
    std::string machine_name;
    std::string manufacturer;
    std::string model;
    std::string serial_number;
    std::string file_version;
    std::string export_date;
    std::string configuration_hash;
    // TEST FIXTURE for the mock v1.1 adapter (docs/migrations/mock_v1_0_to_v1_1.md).
    // Not real schema fields, never serialized — see to_json(MachineConfigMeta)
    // below, which never lists them. Only the mock v1.1 reader/writer
    // (test-only, not part of this header set) ever populates them.
    std::optional<std::string> facility_id;
    std::optional<std::string> config_author;
    ExtraAttrs extra; // non-typed root HDF5 attrs; empty object when none
};

struct OpcuaClientConfig {
    std::string  server_url;
    std::string  auth_mode;
    std::string  security_mode;
    std::string  security_policy;
    std::int64_t bfs_max_depth{0};
    std::int64_t publish_interval{0};
    std::int64_t sampling_interval{0};
    std::int64_t session_timeout{0};
    std::optional<std::int64_t> keep_alive_count;
    std::optional<std::int64_t> lifetime_count;
    std::optional<std::string>  machine_profile;
    std::optional<std::string>  queue_policy;
    std::optional<std::int64_t> queue_size_data_change;
    std::optional<std::int64_t> queue_size_events;
    std::optional<std::int64_t> reconnect_interval;
    std::optional<std::string>  root_node;
    std::optional<std::int64_t> sync_loop_interval_initial;
    std::optional<std::int64_t> sync_loop_interval_settled;
    ExtraAttrs extra;
};

struct OpcuaPipeConfig {
    bool         pipe_enabled{false}; // HDF5 int 0/1
    std::int64_t buffer_size{0};
    std::optional<bool>         configure_client; // HDF5 int 0/1
    std::optional<std::int64_t> inbound_rate_limit;
    std::optional<std::int64_t> max_inbound_message_size;
    std::optional<std::string>  min_integrity_level;
    std::optional<std::string>  pipe_name;
    std::optional<std::string>  user_access_level;
    ExtraAttrs extra;
};

struct OpcuaTrigger {
    std::optional<std::string> id;
    std::optional<std::string> signal;
    std::optional<std::string> subsystem;
    std::optional<bool>        rule_enabled; // HDF5 int 0/1
    std::optional<std::string> start_value;
    std::optional<std::string> stop_value;
    std::optional<std::string>  case_sensitivity;
    std::optional<std::string>  component;
    std::optional<std::int64_t> cooldown_period;
    std::optional<std::string>  event;
    std::optional<std::int64_t> max_fires_per_job;
    std::optional<std::string>  trigger_label;
    ExtraAttrs extra;
};

struct OpcuaConfig {
    OpcuaClientConfig client;
    OpcuaPipeConfig   pipe;
    // key = trigger label (HDF5 sub-group name under OPCUA/Triggers/).
    // std::map is alphabetically ordered; Python dict preserves insertion order.
    // Cross-check uses value comparison so ordering mismatch is benign.
    std::map<std::string, OpcuaTrigger> triggers;
    std::optional<bool> triggers_enabled; // HDF5 float 0.0/1.0 on OPCUA/Triggers group
    std::optional<std::int64_t> trigger_stop_ceiling_layers; // on OPCUA/Triggers group
};

struct MachineConfig {
    MachineConfigMeta         meta;
    Machine                   machine;
    std::vector<OpticalTrain> optical_trains;
    // opcua is OMITTED from JSON entirely when absent (not serialised as null).
    std::optional<OpcuaConfig> opcua;
};

// ---------------------------------------------------------------------------
// JSON serialisation (to_json / from_json)
// Follows ADL pattern; defined inline to keep this a header-only library.
// Field ordering in JSON output is alphabetical (nlohmann default) which may
// differ from Python insertion order; cross_check uses value comparison.
// ---------------------------------------------------------------------------

// --- Grid3D helpers ---

namespace detail {

inline nlohmann::json grid3d_to_json(const Grid3D& grid) {
    nlohmann::json arr = nlohmann::json::array();
    for (const auto& plane : grid) {
        nlohmann::json parr = nlohmann::json::array();
        for (const auto& row : plane) {
            nlohmann::json rarr = nlohmann::json::array();
            for (const auto& cell : row)
                rarr.push_back(cell ? nlohmann::json(*cell) : nlohmann::json(nullptr));
            parr.push_back(std::move(rarr));
        }
        arr.push_back(std::move(parr));
    }
    return arr;
}

inline Grid3D grid3d_from_json(const nlohmann::json& j) {
    Grid3D grid;
    for (const auto& pj : j) {
        std::vector<std::vector<GridCell>> plane;
        for (const auto& rj : pj) {
            std::vector<GridCell> row;
            for (const auto& cj : rj)
                row.push_back(cj.is_null() ? GridCell{} : GridCell{cj.get<double>()});
            plane.push_back(std::move(row));
        }
        grid.push_back(std::move(plane));
    }
    return grid;
}

} // namespace detail

// --- ScanFieldCorrectionFile ---

inline void to_json(nlohmann::json& j, const ScanFieldCorrectionFile& s) {
    j = {
        {"document_created_at", detail::opt_to_j(s.document_created_at)},
        {"document_id",         s.document_id},
        {"document_name",       s.document_name},
        {"document_type",       detail::opt_to_j(s.document_type)},
        {"file_size",           s.file_size},
        {"original_uri",        detail::opt_to_j(s.original_uri)},
        {"valid_as_of_date",    s.valid_as_of_date},
    };
    // raw_bytes: base64-encoded string when present; omitted otherwise.
    // Base64 encoding implemented in the reader when include_binary=true (§4.11).
    if (s.raw_bytes.has_value())
        j["raw_bytes"] = nullptr; // placeholder; replaced in §4.11
}

inline void from_json(const nlohmann::json& j, ScanFieldCorrectionFile& s) {
    j.at("document_name").get_to(s.document_name);
    j.at("document_id").get_to(s.document_id);
    j.at("file_size").get_to(s.file_size);
    j.at("valid_as_of_date").get_to(s.valid_as_of_date);
    s.document_created_at = detail::j_to_opt<std::string>(j, "document_created_at");
    s.document_type       = detail::j_to_opt<std::string>(j, "document_type");
    s.original_uri        = detail::j_to_opt<std::string>(j, "original_uri");
    // raw_bytes base64 decode implemented in the writer (§4.14).
}

// --- EquationConstant / CalibrationPoint / SynchronousSensor ---

inline void to_json(nlohmann::json& j, const EquationConstant& c) {
    j = {{"name", c.name}, {"value", c.value}};
}

inline void from_json(const nlohmann::json& j, EquationConstant& c) {
    j.at("name").get_to(c.name);
    j.at("value").get_to(c.value);
}

inline void to_json(nlohmann::json& j, const CalibrationPoint& c) {
    j = {{"input_value", c.input_value}, {"output_value", c.output_value}};
}

inline void from_json(const nlohmann::json& j, CalibrationPoint& c) {
    j.at("input_value").get_to(c.input_value);
    j.at("output_value").get_to(c.output_value);
}

inline void to_json(nlohmann::json& j, const SynchronousSensor& s) {
    j = {
        {"enabled",                       detail::opt_to_j(s.enabled)},
        {"sensor_name",                   detail::opt_to_j(s.sensor_name)},
        {"sensor_output_range_low",       detail::opt_to_j(s.sensor_output_range_low)},
        {"sensor_output_range_high",      detail::opt_to_j(s.sensor_output_range_high)},
        {"sensor_output_space",           detail::opt_to_j(s.sensor_output_space)},
        {"sensor_model",                  detail::opt_to_j(s.sensor_model)},
        {"sensor_manufacturer",           detail::opt_to_j(s.sensor_manufacturer)},
        {"sensor_scope",                  detail::opt_to_j(s.sensor_scope)},
        {"units_derived_quantity",        detail::opt_to_j(s.units_derived_quantity)},
        {"port_id",                       detail::opt_to_j(s.port_id)},
        {"sensor_type",                   detail::opt_to_j(s.sensor_type)},
        {"input_type",                    detail::opt_to_j(s.input_type)},
        {"algorithm_type",                detail::opt_to_j(s.algorithm_type)},
        {"algorithm_equation",            detail::opt_to_j(s.algorithm_equation)},
        {"calibration_source",            detail::opt_to_j(s.calibration_source)},
        {"calibration_verified",          detail::opt_to_j(s.calibration_verified)},
        {"sample_period",                 detail::opt_to_j(s.sample_period)},
        {"metadata",                      detail::opt_to_j(s.metadata)},
        {"derivation_equation_constants", s.derivation_equation_constants},
        {"calibration_points",            s.calibration_points},
    };
}

inline void from_json(const nlohmann::json& j, SynchronousSensor& s) {
    s.enabled                  = detail::j_to_opt<bool>(j, "enabled");
    s.sensor_name               = detail::j_to_opt<std::string>(j, "sensor_name");
    s.sensor_output_range_low   = detail::j_to_opt<double>(j, "sensor_output_range_low");
    s.sensor_output_range_high  = detail::j_to_opt<double>(j, "sensor_output_range_high");
    s.sensor_output_space       = detail::j_to_opt<std::string>(j, "sensor_output_space");
    s.sensor_model              = detail::j_to_opt<std::string>(j, "sensor_model");
    s.sensor_manufacturer       = detail::j_to_opt<std::string>(j, "sensor_manufacturer");
    s.sensor_scope              = detail::j_to_opt<std::string>(j, "sensor_scope");
    s.units_derived_quantity    = detail::j_to_opt<std::string>(j, "units_derived_quantity");
    s.port_id                   = detail::j_to_opt<std::int64_t>(j, "port_id");
    s.sensor_type               = detail::j_to_opt<std::string>(j, "sensor_type");
    s.input_type                = detail::j_to_opt<std::string>(j, "input_type");
    s.algorithm_type            = detail::j_to_opt<std::string>(j, "algorithm_type");
    s.algorithm_equation        = detail::j_to_opt<std::string>(j, "algorithm_equation");
    s.calibration_source        = detail::j_to_opt<std::string>(j, "calibration_source");
    s.calibration_verified      = detail::j_to_opt<bool>(j, "calibration_verified");
    s.sample_period             = detail::j_to_opt<double>(j, "sample_period");
    s.metadata                  = detail::j_to_opt<std::string>(j, "metadata");
    j.at("derivation_equation_constants").get_to(s.derivation_equation_constants);
    j.at("calibration_points").get_to(s.calibration_points);
}

// --- ClearBox ---

inline void to_json(nlohmann::json& j, const ClearBox& c) {
    j = {
        {"actual_timing_offset",         detail::opt_to_j(c.actual_timing_offset)},
        {"commanded_timing_offset",      detail::opt_to_j(c.commanded_timing_offset)},
        {"correction_grid_domain_shape", detail::opt_to_j(c.correction_grid_domain_shape)},
        {"custom_video_format",          detail::opt_to_j(c.custom_video_format)},
        {"data_port",                    detail::opt_to_j(c.data_port)},
        {"inverse_grid_domain_shape",    detail::opt_to_j(c.inverse_grid_domain_shape)},
        {"ip_address",                   c.ip_address},
        {"manufacturer",                 detail::opt_to_j(c.manufacturer)},
        {"model",                        detail::opt_to_j(c.model)},
        {"output_path",                  detail::opt_to_j(c.output_path)},
        {"selected_camera",              detail::opt_to_j(c.selected_camera)},
        {"serial_number",                detail::opt_to_j(c.serial_number)},
        {"server_port",                  detail::opt_to_j(c.server_port)},
        {"show_console",                 detail::opt_to_j(c.show_console)},
        {"software_trigger_delay",       detail::opt_to_j(c.software_trigger_delay)},
        {"video_output",                 detail::opt_to_j(c.video_output)},
        {"volts_to_watts_algorithm",     detail::opt_to_j(c.volts_to_watts_algorithm)},
        {"volts_to_watts_params",        detail::opt_to_j(c.volts_to_watts_params)},
    };
    // Correction grids: omitted when absent (include_binary=false).
    if (c.correction_data.has_value())
        j["correction_data"] = detail::grid3d_to_json(*c.correction_data);
    if (c.inverse_correction_data.has_value())
        j["inverse_correction_data"] = detail::grid3d_to_json(*c.inverse_correction_data);
    // synchronous_sensors: omitted entirely when empty, not serialised as
    // "{}" — see the field's doc comment on ClearBox above.
    if (!c.synchronous_sensors.empty())
        j["synchronous_sensors"] = c.synchronous_sensors;
}

inline void from_json(const nlohmann::json& j, ClearBox& c) {
    j.at("ip_address").get_to(c.ip_address);
    c.serial_number           = detail::j_to_opt<std::string>(j, "serial_number");
    c.data_port               = detail::j_to_opt<std::int64_t>(j, "data_port");
    c.server_port             = detail::j_to_opt<std::int64_t>(j, "server_port");
    c.actual_timing_offset    = detail::j_to_opt<std::int64_t>(j, "actual_timing_offset");
    c.commanded_timing_offset = detail::j_to_opt<std::int64_t>(j, "commanded_timing_offset");
    c.manufacturer            = detail::j_to_opt<std::string>(j, "manufacturer");
    c.model                   = detail::j_to_opt<std::string>(j, "model");
    c.output_path             = detail::j_to_opt<std::string>(j, "output_path");
    c.selected_camera         = detail::j_to_opt<std::string>(j, "selected_camera");
    c.custom_video_format     = detail::j_to_opt<std::string>(j, "custom_video_format");
    c.video_output            = detail::j_to_opt<std::string>(j, "video_output");
    c.show_console            = detail::j_to_opt<bool>(j, "show_console");
    c.software_trigger_delay  = detail::j_to_opt<std::int64_t>(j, "software_trigger_delay");
    c.volts_to_watts_algorithm     = detail::j_to_opt<std::string>(j, "volts_to_watts_algorithm");
    c.volts_to_watts_params        = detail::j_to_opt<std::string>(j, "volts_to_watts_params");
    c.correction_grid_domain_shape = detail::j_to_opt<std::string>(j, "correction_grid_domain_shape");
    c.inverse_grid_domain_shape    = detail::j_to_opt<std::string>(j, "inverse_grid_domain_shape");
    if (j.contains("correction_data") && !j.at("correction_data").is_null())
        c.correction_data = detail::grid3d_from_json(j.at("correction_data"));
    if (j.contains("inverse_correction_data") && !j.at("inverse_correction_data").is_null())
        c.inverse_correction_data = detail::grid3d_from_json(j.at("inverse_correction_data"));
    c.synchronous_sensors.clear();
    if (j.contains("synchronous_sensors"))
        j.at("synchronous_sensors").get_to(c.synchronous_sensors);
}

// --- OptionalComponents ---

inline void to_json(nlohmann::json& j, const OptionalComponents& o) {
    j = {{"clearbox", detail::opt_to_j(o.clearbox)}};
}

inline void from_json(const nlohmann::json& j, OptionalComponents& o) {
    if (j.contains("clearbox") && !j.at("clearbox").is_null())
        o.clearbox = j.at("clearbox").get<ClearBox>();
    else
        o.clearbox = std::nullopt;
}

// --- Collimator ---

inline void to_json(nlohmann::json& j, const Collimator& c) {
    j = {
        {"focal_length",      detail::opt_to_j(c.focal_length)},
        {"focal_length_unit", detail::opt_to_j(c.focal_length_unit)},
        {"manufacturer",      c.manufacturer},
        {"model",             c.model},
        {"serial_number",     c.serial_number},
    };
}

inline void from_json(const nlohmann::json& j, Collimator& c) {
    j.at("manufacturer").get_to(c.manufacturer);
    j.at("model").get_to(c.model);
    j.at("serial_number").get_to(c.serial_number);
    c.focal_length      = detail::j_to_opt<double>(j, "focal_length");
    c.focal_length_unit = detail::j_to_opt<std::string>(j, "focal_length_unit");
}

// --- ScannerCard ---

inline void to_json(nlohmann::json& j, const ScannerCard& s) {
    j = {
        {"communication_protocol", detail::opt_to_j(s.communication_protocol)},
        {"manufacturer",           s.manufacturer},
        {"model",                  s.model},
        {"sample_period",          detail::opt_to_j(s.sample_period)},
        {"sample_period_unit",     detail::opt_to_j(s.sample_period_unit)},
        {"serial_number",          s.serial_number},
    };
}

inline void from_json(const nlohmann::json& j, ScannerCard& s) {
    j.at("manufacturer").get_to(s.manufacturer);
    j.at("model").get_to(s.model);
    j.at("serial_number").get_to(s.serial_number);
    s.communication_protocol = detail::j_to_opt<std::string>(j, "communication_protocol");
    s.sample_period          = detail::j_to_opt<double>(j, "sample_period");
    s.sample_period_unit     = detail::j_to_opt<std::string>(j, "sample_period_unit");
}

// --- AxisConfig ---

inline void to_json(nlohmann::json& j, const AxisConfig& a) {
    j = {
        {"actual_bit_resolution",       detail::opt_to_j(a.actual_bit_resolution)},
        {"actual_bit_resolution_unit",  detail::opt_to_j(a.actual_bit_resolution_unit)},
        {"commanded_bit_resolution",    detail::opt_to_j(a.commanded_bit_resolution)},
        {"commanded_bit_resolution_unit",detail::opt_to_j(a.commanded_bit_resolution_unit)},
        {"control_type",                detail::opt_to_j(a.control_type)},
        {"range_of_motion",             detail::opt_to_j(a.range_of_motion)},
        {"range_of_motion_unit",        detail::opt_to_j(a.range_of_motion_unit)},
        {"smoothing_kernel",            detail::opt_to_j(a.smoothing_kernel)},
        {"smoothing_parameters",        detail::opt_to_j(a.smoothing_parameters)},
        {"tuning_parameters",           detail::opt_to_j(a.tuning_parameters)},
        {"tuning_type",                 detail::opt_to_j(a.tuning_type)},
    };
}

inline void from_json(const nlohmann::json& j, AxisConfig& a) {
    a.actual_bit_resolution       = detail::j_to_opt<std::int64_t>(j, "actual_bit_resolution");
    a.actual_bit_resolution_unit  = detail::j_to_opt<std::string>(j, "actual_bit_resolution_unit");
    a.commanded_bit_resolution    = detail::j_to_opt<std::int64_t>(j, "commanded_bit_resolution");
    a.commanded_bit_resolution_unit = detail::j_to_opt<std::string>(j, "commanded_bit_resolution_unit");
    a.control_type        = detail::j_to_opt<std::string>(j, "control_type");
    a.range_of_motion     = detail::j_to_opt<double>(j, "range_of_motion");
    a.range_of_motion_unit= detail::j_to_opt<std::string>(j, "range_of_motion_unit");
    a.smoothing_kernel    = detail::j_to_opt<std::string>(j, "smoothing_kernel");
    a.smoothing_parameters= detail::j_to_opt<double>(j, "smoothing_parameters");
    a.tuning_parameters   = detail::j_to_opt<std::string>(j, "tuning_parameters");
    a.tuning_type         = detail::j_to_opt<std::string>(j, "tuning_type");
}

// --- Scanner ---

inline void to_json(nlohmann::json& j, const Scanner& s) {
    j = {
        {"axis_configuration",      detail::opt_to_j(s.axis_configuration)},
        {"focus",                   detail::opt_to_j(s.focus)},
        {"manufacturer",            s.manufacturer},
        {"model",                   s.model},
        {"scan_field_x",            detail::opt_to_j(s.scan_field_x)},
        {"scan_field_x_unit",       detail::opt_to_j(s.scan_field_x_unit)},
        {"scan_field_y",            detail::opt_to_j(s.scan_field_y)},
        {"scan_field_y_unit",       detail::opt_to_j(s.scan_field_y_unit)},
        {"scan_field_z",            detail::opt_to_j(s.scan_field_z)},
        {"scan_field_z_unit",       detail::opt_to_j(s.scan_field_z_unit)},
        {"scan_head_offset_x",      detail::opt_to_j(s.scan_head_offset_x)},
        {"scan_head_offset_x_unit", detail::opt_to_j(s.scan_head_offset_x_unit)},
        {"scan_head_offset_y",      detail::opt_to_j(s.scan_head_offset_y)},
        {"scan_head_offset_y_unit", detail::opt_to_j(s.scan_head_offset_y_unit)},
        {"scan_head_offset_z",      detail::opt_to_j(s.scan_head_offset_z)},
        {"scan_head_offset_z_unit", detail::opt_to_j(s.scan_head_offset_z_unit)},
        {"scan_head_rotation",      detail::opt_to_j(s.scan_head_rotation)},
        {"scan_head_rotation_unit", detail::opt_to_j(s.scan_head_rotation_unit)},
        {"serial_number",           s.serial_number},
        {"working_distance",        detail::opt_to_j(s.working_distance)},
        {"working_distance_unit",   detail::opt_to_j(s.working_distance_unit)},
        {"x_axis",                  s.x_axis},
        {"y_axis",                  s.y_axis},
        {"z_axis",                  detail::opt_to_j(s.z_axis)},
    };
}

inline void from_json(const nlohmann::json& j, Scanner& s) {
    j.at("manufacturer").get_to(s.manufacturer);
    j.at("model").get_to(s.model);
    j.at("serial_number").get_to(s.serial_number);
    s.working_distance       = detail::j_to_opt<double>(j, "working_distance");
    s.working_distance_unit  = detail::j_to_opt<std::string>(j, "working_distance_unit");
    s.scan_field_x           = detail::j_to_opt<double>(j, "scan_field_x");
    s.scan_field_x_unit      = detail::j_to_opt<std::string>(j, "scan_field_x_unit");
    s.scan_field_y           = detail::j_to_opt<double>(j, "scan_field_y");
    s.scan_field_y_unit      = detail::j_to_opt<std::string>(j, "scan_field_y_unit");
    s.scan_field_z           = detail::j_to_opt<double>(j, "scan_field_z");
    s.scan_field_z_unit      = detail::j_to_opt<std::string>(j, "scan_field_z_unit");
    s.scan_head_offset_x     = detail::j_to_opt<double>(j, "scan_head_offset_x");
    s.scan_head_offset_x_unit= detail::j_to_opt<std::string>(j, "scan_head_offset_x_unit");
    s.scan_head_offset_y     = detail::j_to_opt<double>(j, "scan_head_offset_y");
    s.scan_head_offset_y_unit= detail::j_to_opt<std::string>(j, "scan_head_offset_y_unit");
    s.scan_head_offset_z     = detail::j_to_opt<double>(j, "scan_head_offset_z");
    s.scan_head_offset_z_unit= detail::j_to_opt<std::string>(j, "scan_head_offset_z_unit");
    s.scan_head_rotation     = detail::j_to_opt<double>(j, "scan_head_rotation");
    s.scan_head_rotation_unit= detail::j_to_opt<std::string>(j, "scan_head_rotation_unit");
    s.axis_configuration     = detail::j_to_opt<std::string>(j, "axis_configuration");
    j.at("x_axis").get_to(s.x_axis);
    j.at("y_axis").get_to(s.y_axis);
    if (j.contains("z_axis") && !j.at("z_axis").is_null())
        s.z_axis = j.at("z_axis").get<AxisConfig>();
    if (j.contains("focus") && !j.at("focus").is_null())
        s.focus = j.at("focus").get<AxisConfig>();
}

// --- LightSource ---

inline void to_json(nlohmann::json& j, const LightSource& l) {
    j = {
        {"manufacturer",              l.manufacturer},
        {"model",                     l.model},
        {"power_bit_resolution",      detail::opt_to_j(l.power_bit_resolution)},
        {"power_bit_resolution_unit", detail::opt_to_j(l.power_bit_resolution_unit)},
        {"power_max_actual",          detail::opt_to_j(l.power_max_actual)},
        {"power_max_actual_unit",     detail::opt_to_j(l.power_max_actual_unit)},
        {"power_max_nominal",         detail::opt_to_j(l.power_max_nominal)},
        {"power_max_nominal_unit",    detail::opt_to_j(l.power_max_nominal_unit)},
        {"power_min_actual",          detail::opt_to_j(l.power_min_actual)},
        {"power_min_actual_unit",     detail::opt_to_j(l.power_min_actual_unit)},
        {"power_min_nominal",         detail::opt_to_j(l.power_min_nominal)},
        {"power_min_nominal_unit",    detail::opt_to_j(l.power_min_nominal_unit)},
        {"serial_number",             l.serial_number},
        {"watts_to_volts_algorithm",  detail::opt_to_j(l.watts_to_volts_algorithm)},
        {"watts_to_volts_params",     detail::opt_to_j(l.watts_to_volts_params)},
        {"wavelength",                detail::opt_to_j(l.wavelength)},
        {"wavelength_unit",           detail::opt_to_j(l.wavelength_unit)},
    };
}

inline void from_json(const nlohmann::json& j, LightSource& l) {
    j.at("manufacturer").get_to(l.manufacturer);
    j.at("model").get_to(l.model);
    j.at("serial_number").get_to(l.serial_number);
    l.wavelength              = detail::j_to_opt<double>(j, "wavelength");
    l.wavelength_unit         = detail::j_to_opt<std::string>(j, "wavelength_unit");
    l.power_max_nominal       = detail::j_to_opt<double>(j, "power_max_nominal");
    l.power_max_nominal_unit  = detail::j_to_opt<std::string>(j, "power_max_nominal_unit");
    l.power_max_actual        = detail::j_to_opt<double>(j, "power_max_actual");
    l.power_max_actual_unit   = detail::j_to_opt<std::string>(j, "power_max_actual_unit");
    l.power_min_actual        = detail::j_to_opt<double>(j, "power_min_actual");
    l.power_min_actual_unit   = detail::j_to_opt<std::string>(j, "power_min_actual_unit");
    l.power_min_nominal       = detail::j_to_opt<double>(j, "power_min_nominal");
    l.power_min_nominal_unit  = detail::j_to_opt<std::string>(j, "power_min_nominal_unit");
    l.power_bit_resolution    = detail::j_to_opt<double>(j, "power_bit_resolution");
    l.power_bit_resolution_unit= detail::j_to_opt<std::string>(j, "power_bit_resolution_unit");
    l.watts_to_volts_algorithm= detail::j_to_opt<std::string>(j, "watts_to_volts_algorithm");
    l.watts_to_volts_params   = detail::j_to_opt<std::string>(j, "watts_to_volts_params");
}

// --- OpticalTrain ---

inline void to_json(nlohmann::json& j, const OpticalTrain& t) {
    j = {
        {"beam_profile_type",                    detail::opt_to_j(t.beam_profile_type)},
        {"beam_waist_definition",                detail::opt_to_j(t.beam_waist_definition)},
        {"beam_waist_major",                     detail::opt_to_j(t.beam_waist_major)},
        {"beam_waist_major_unit",                detail::opt_to_j(t.beam_waist_major_unit)},
        {"beam_waist_minor",                     detail::opt_to_j(t.beam_waist_minor)},
        {"beam_waist_minor_unit",                detail::opt_to_j(t.beam_waist_minor_unit)},
        {"beam_waist_offset_z",                  detail::opt_to_j(t.beam_waist_offset_z)},
        {"beam_waist_offset_z_unit",             detail::opt_to_j(t.beam_waist_offset_z_unit)},
        {"build_plane_offset_major",             detail::opt_to_j(t.build_plane_offset_major)},
        {"build_plane_offset_major_unit",        detail::opt_to_j(t.build_plane_offset_major_unit)},
        {"build_plane_offset_minor",             detail::opt_to_j(t.build_plane_offset_minor)},
        {"build_plane_offset_minor_unit",        detail::opt_to_j(t.build_plane_offset_minor_unit)},
        {"collimator",                           t.collimator},
        {"collimator_focal_length",              detail::opt_to_j(t.collimator_focal_length)},
        {"collimator_focal_length_unit",         detail::opt_to_j(t.collimator_focal_length_unit)},
        {"id",                                   detail::opt_to_j(t.id)},
        {"light_source",                         t.light_source},
        {"m2_major",                             detail::opt_to_j(t.m2_major)},
        {"m2_minor",                             detail::opt_to_j(t.m2_minor)},
        {"major_axis_angle",                     detail::opt_to_j(t.major_axis_angle)},
        {"major_axis_angle_unit",                detail::opt_to_j(t.major_axis_angle_unit)},
        {"optional_components",                  t.optional_components},
        {"rayleigh_length_major",                detail::opt_to_j(t.rayleigh_length_major)},
        {"rayleigh_length_major_unit",           detail::opt_to_j(t.rayleigh_length_major_unit)},
        {"rayleigh_length_minor",                detail::opt_to_j(t.rayleigh_length_minor)},
        {"rayleigh_length_minor_unit",           detail::opt_to_j(t.rayleigh_length_minor_unit)},
        {"scan_field_correction_file",           detail::opt_to_j(t.scan_field_correction_file)},
        {"scanner",                              t.scanner},
        {"scanner_card",                         t.scanner_card},
        {"scanner_number",                       detail::opt_to_j(t.scanner_number)},
        {"thermal_lensing_focal_plane_shift",    detail::opt_to_j(t.thermal_lensing_focal_plane_shift)},
        {"thermal_lensing_focal_plane_shift_unit",detail::opt_to_j(t.thermal_lensing_focal_plane_shift_unit)},
        {"thermal_lensing_passed",               detail::opt_to_j(t.thermal_lensing_passed)},
        {"thermal_lensing_threshold",            detail::opt_to_j(t.thermal_lensing_threshold)},
        {"thermal_lensing_threshold_unit",       detail::opt_to_j(t.thermal_lensing_threshold_unit)},
        {"train_id",                             t.train_id},
    };
}

inline void from_json(const nlohmann::json& j, OpticalTrain& t) {
    j.at("train_id").get_to(t.train_id);
    t.id                    = detail::j_to_opt<std::string>(j, "id");
    t.beam_profile_type     = detail::j_to_opt<std::string>(j, "beam_profile_type");
    t.beam_waist_definition = detail::j_to_opt<std::string>(j, "beam_waist_definition");
    t.beam_waist_major      = detail::j_to_opt<double>(j, "beam_waist_major");
    t.beam_waist_major_unit = detail::j_to_opt<std::string>(j, "beam_waist_major_unit");
    t.beam_waist_minor      = detail::j_to_opt<double>(j, "beam_waist_minor");
    t.beam_waist_minor_unit = detail::j_to_opt<std::string>(j, "beam_waist_minor_unit");
    t.beam_waist_offset_z        = detail::j_to_opt<double>(j, "beam_waist_offset_z");
    t.beam_waist_offset_z_unit   = detail::j_to_opt<std::string>(j, "beam_waist_offset_z_unit");
    t.build_plane_offset_major   = detail::j_to_opt<double>(j, "build_plane_offset_major");
    t.build_plane_offset_major_unit = detail::j_to_opt<std::string>(j, "build_plane_offset_major_unit");
    t.build_plane_offset_minor   = detail::j_to_opt<double>(j, "build_plane_offset_minor");
    t.build_plane_offset_minor_unit = detail::j_to_opt<std::string>(j, "build_plane_offset_minor_unit");
    t.collimator_focal_length    = detail::j_to_opt<double>(j, "collimator_focal_length");
    t.collimator_focal_length_unit= detail::j_to_opt<std::string>(j, "collimator_focal_length_unit");
    t.m2_major              = detail::j_to_opt<double>(j, "m2_major");
    t.m2_minor              = detail::j_to_opt<double>(j, "m2_minor");
    t.major_axis_angle      = detail::j_to_opt<double>(j, "major_axis_angle");
    t.major_axis_angle_unit = detail::j_to_opt<std::string>(j, "major_axis_angle_unit");
    t.rayleigh_length_major      = detail::j_to_opt<double>(j, "rayleigh_length_major");
    t.rayleigh_length_major_unit = detail::j_to_opt<std::string>(j, "rayleigh_length_major_unit");
    t.rayleigh_length_minor      = detail::j_to_opt<double>(j, "rayleigh_length_minor");
    t.rayleigh_length_minor_unit = detail::j_to_opt<std::string>(j, "rayleigh_length_minor_unit");
    t.scanner_number             = detail::j_to_opt<std::string>(j, "scanner_number");
    t.thermal_lensing_passed     = detail::j_to_opt<bool>(j, "thermal_lensing_passed");
    t.thermal_lensing_focal_plane_shift      = detail::j_to_opt<double>(j, "thermal_lensing_focal_plane_shift");
    t.thermal_lensing_focal_plane_shift_unit = detail::j_to_opt<std::string>(j, "thermal_lensing_focal_plane_shift_unit");
    t.thermal_lensing_threshold      = detail::j_to_opt<double>(j, "thermal_lensing_threshold");
    t.thermal_lensing_threshold_unit = detail::j_to_opt<std::string>(j, "thermal_lensing_threshold_unit");
    j.at("scanner").get_to(t.scanner);
    j.at("light_source").get_to(t.light_source);
    j.at("collimator").get_to(t.collimator);
    j.at("scanner_card").get_to(t.scanner_card);
    j.at("optional_components").get_to(t.optional_components);
    if (j.contains("scan_field_correction_file") && !j.at("scan_field_correction_file").is_null())
        t.scan_field_correction_file = j.at("scan_field_correction_file").get<ScanFieldCorrectionFile>();
}

// --- Machine (flat — no nested build_plate sub-object) ---

inline void to_json(nlohmann::json& j, const Machine& m) {
    j = {
        {"build_plate_radius",      detail::opt_to_j(m.build_plate_radius)},
        {"build_plate_radius_unit", detail::opt_to_j(m.build_plate_radius_unit)},
        {"build_plate_x",           detail::opt_to_j(m.build_plate_x)},
        {"build_plate_x_unit",      detail::opt_to_j(m.build_plate_x_unit)},
        {"build_plate_y",           detail::opt_to_j(m.build_plate_y)},
        {"build_plate_y_unit",      detail::opt_to_j(m.build_plate_y_unit)},
        {"build_plate_z",           detail::opt_to_j(m.build_plate_z)},
        {"build_plate_z_unit",      detail::opt_to_j(m.build_plate_z_unit)},
        {"gas_flow_direction",      detail::opt_to_j(m.gas_flow_direction)},
        {"id",                      detail::opt_to_j(m.id)},
        {"machine_name",            m.machine_name},
        {"manufacturer",            m.manufacturer},
        {"model",                   m.model},
        {"recoat_direction",        detail::opt_to_j(m.recoat_direction)},
        {"serial_number",           m.serial_number},
    };
}

inline void from_json(const nlohmann::json& j, Machine& m) {
    j.at("machine_name").get_to(m.machine_name);
    j.at("manufacturer").get_to(m.manufacturer);
    j.at("model").get_to(m.model);
    j.at("serial_number").get_to(m.serial_number);
    m.id                   = detail::j_to_opt<std::string>(j, "id");
    m.build_plate_x        = detail::j_to_opt<double>(j, "build_plate_x");
    m.build_plate_x_unit   = detail::j_to_opt<std::string>(j, "build_plate_x_unit");
    m.build_plate_y        = detail::j_to_opt<double>(j, "build_plate_y");
    m.build_plate_y_unit   = detail::j_to_opt<std::string>(j, "build_plate_y_unit");
    m.build_plate_z        = detail::j_to_opt<double>(j, "build_plate_z");
    m.build_plate_z_unit   = detail::j_to_opt<std::string>(j, "build_plate_z_unit");
    m.build_plate_radius   = detail::j_to_opt<double>(j, "build_plate_radius");
    m.build_plate_radius_unit = detail::j_to_opt<std::string>(j, "build_plate_radius_unit");
    m.gas_flow_direction   = detail::j_to_opt<std::string>(j, "gas_flow_direction");
    m.recoat_direction     = detail::j_to_opt<std::string>(j, "recoat_direction");
}

// --- MachineConfigMeta ---

inline void to_json(nlohmann::json& j, const MachineConfigMeta& m) {
    j = {
        {"configuration_hash", m.configuration_hash},
        {"export_date",        m.export_date},
        {"extra",              m.extra},
        {"file_version",       m.file_version},
        {"machine_name",       m.machine_name},
        {"manufacturer",       m.manufacturer},
        {"model",              m.model},
        {"schema_version",     m.schema_version},
        {"serial_number",      m.serial_number},
    };
}

inline void from_json(const nlohmann::json& j, MachineConfigMeta& m) {
    j.at("schema_version").get_to(m.schema_version);
    j.at("machine_name").get_to(m.machine_name);
    j.at("manufacturer").get_to(m.manufacturer);
    j.at("model").get_to(m.model);
    j.at("serial_number").get_to(m.serial_number);
    j.at("file_version").get_to(m.file_version);
    j.at("export_date").get_to(m.export_date);
    j.at("configuration_hash").get_to(m.configuration_hash);
    m.extra = (j.contains("extra") && j.at("extra").is_object())
              ? j.at("extra") : nlohmann::json::object();
}

// --- OpcuaClientConfig ---

inline void to_json(nlohmann::json& j, const OpcuaClientConfig& o) {
    j = {
        {"auth_mode",                    o.auth_mode},
        {"bfs_max_depth",                o.bfs_max_depth},
        {"extra",                        o.extra},
        {"keep_alive_count",             detail::opt_to_j(o.keep_alive_count)},
        {"lifetime_count",               detail::opt_to_j(o.lifetime_count)},
        {"machine_profile",              detail::opt_to_j(o.machine_profile)},
        {"publish_interval",             o.publish_interval},
        {"queue_policy",                 detail::opt_to_j(o.queue_policy)},
        {"queue_size_data_change",       detail::opt_to_j(o.queue_size_data_change)},
        {"queue_size_events",            detail::opt_to_j(o.queue_size_events)},
        {"reconnect_interval",           detail::opt_to_j(o.reconnect_interval)},
        {"root_node",                    detail::opt_to_j(o.root_node)},
        {"sampling_interval",            o.sampling_interval},
        {"security_mode",                o.security_mode},
        {"security_policy",              o.security_policy},
        {"server_url",                   o.server_url},
        {"session_timeout",              o.session_timeout},
        {"sync_loop_interval_initial",   detail::opt_to_j(o.sync_loop_interval_initial)},
        {"sync_loop_interval_settled",   detail::opt_to_j(o.sync_loop_interval_settled)},
    };
}

inline void from_json(const nlohmann::json& j, OpcuaClientConfig& o) {
    j.at("server_url").get_to(o.server_url);
    j.at("auth_mode").get_to(o.auth_mode);
    j.at("security_mode").get_to(o.security_mode);
    j.at("security_policy").get_to(o.security_policy);
    j.at("bfs_max_depth").get_to(o.bfs_max_depth);
    j.at("publish_interval").get_to(o.publish_interval);
    j.at("sampling_interval").get_to(o.sampling_interval);
    j.at("session_timeout").get_to(o.session_timeout);
    o.keep_alive_count             = detail::j_to_opt<std::int64_t>(j, "keep_alive_count");
    o.lifetime_count               = detail::j_to_opt<std::int64_t>(j, "lifetime_count");
    o.machine_profile               = detail::j_to_opt<std::string>(j, "machine_profile");
    o.queue_policy                  = detail::j_to_opt<std::string>(j, "queue_policy");
    o.queue_size_data_change       = detail::j_to_opt<std::int64_t>(j, "queue_size_data_change");
    o.queue_size_events             = detail::j_to_opt<std::int64_t>(j, "queue_size_events");
    o.reconnect_interval             = detail::j_to_opt<std::int64_t>(j, "reconnect_interval");
    o.root_node                     = detail::j_to_opt<std::string>(j, "root_node");
    o.sync_loop_interval_initial   = detail::j_to_opt<std::int64_t>(j, "sync_loop_interval_initial");
    o.sync_loop_interval_settled   = detail::j_to_opt<std::int64_t>(j, "sync_loop_interval_settled");
    o.extra = (j.contains("extra") && j.at("extra").is_object())
              ? j.at("extra") : nlohmann::json::object();
}

// --- OpcuaPipeConfig ---

inline void to_json(nlohmann::json& j, const OpcuaPipeConfig& o) {
    j = {
        {"buffer_size",              o.buffer_size},
        {"configure_client",         detail::opt_to_j(o.configure_client)},
        {"extra",                    o.extra},
        {"inbound_rate_limit",       detail::opt_to_j(o.inbound_rate_limit)},
        {"max_inbound_message_size", detail::opt_to_j(o.max_inbound_message_size)},
        {"min_integrity_level",      detail::opt_to_j(o.min_integrity_level)},
        {"pipe_enabled",             o.pipe_enabled},
        {"pipe_name",                detail::opt_to_j(o.pipe_name)},
        {"user_access_level",        detail::opt_to_j(o.user_access_level)},
    };
}

inline void from_json(const nlohmann::json& j, OpcuaPipeConfig& o) {
    j.at("pipe_enabled").get_to(o.pipe_enabled);
    j.at("buffer_size").get_to(o.buffer_size);
    o.configure_client         = detail::j_to_opt<bool>(j, "configure_client");
    o.inbound_rate_limit       = detail::j_to_opt<std::int64_t>(j, "inbound_rate_limit");
    o.max_inbound_message_size = detail::j_to_opt<std::int64_t>(j, "max_inbound_message_size");
    o.min_integrity_level      = detail::j_to_opt<std::string>(j, "min_integrity_level");
    o.pipe_name                = detail::j_to_opt<std::string>(j, "pipe_name");
    o.user_access_level        = detail::j_to_opt<std::string>(j, "user_access_level");
    o.extra = (j.contains("extra") && j.at("extra").is_object())
              ? j.at("extra") : nlohmann::json::object();
}

// --- OpcuaTrigger ---

inline void to_json(nlohmann::json& j, const OpcuaTrigger& o) {
    j = {
        {"case_sensitivity", detail::opt_to_j(o.case_sensitivity)},
        {"component",        detail::opt_to_j(o.component)},
        {"cooldown_period",  detail::opt_to_j(o.cooldown_period)},
        {"event",            detail::opt_to_j(o.event)},
        {"extra",            o.extra},
        {"id",               detail::opt_to_j(o.id)},
        {"max_fires_per_job", detail::opt_to_j(o.max_fires_per_job)},
        {"rule_enabled",     detail::opt_to_j(o.rule_enabled)},
        {"signal",           detail::opt_to_j(o.signal)},
        {"start_value",      detail::opt_to_j(o.start_value)},
        {"stop_value",       detail::opt_to_j(o.stop_value)},
        {"subsystem",        detail::opt_to_j(o.subsystem)},
        {"trigger_label",    detail::opt_to_j(o.trigger_label)},
    };
}

inline void from_json(const nlohmann::json& j, OpcuaTrigger& o) {
    o.id           = detail::j_to_opt<std::string>(j, "id");
    o.signal       = detail::j_to_opt<std::string>(j, "signal");
    o.subsystem    = detail::j_to_opt<std::string>(j, "subsystem");
    o.rule_enabled = detail::j_to_opt<bool>(j, "rule_enabled");
    o.start_value  = detail::j_to_opt<std::string>(j, "start_value");
    o.stop_value   = detail::j_to_opt<std::string>(j, "stop_value");
    o.case_sensitivity = detail::j_to_opt<std::string>(j, "case_sensitivity");
    o.component        = detail::j_to_opt<std::string>(j, "component");
    o.cooldown_period   = detail::j_to_opt<std::int64_t>(j, "cooldown_period");
    o.event             = detail::j_to_opt<std::string>(j, "event");
    o.max_fires_per_job = detail::j_to_opt<std::int64_t>(j, "max_fires_per_job");
    o.trigger_label     = detail::j_to_opt<std::string>(j, "trigger_label");
    o.extra = (j.contains("extra") && j.at("extra").is_object())
              ? j.at("extra") : nlohmann::json::object();
}

// --- OpcuaConfig ---

inline void to_json(nlohmann::json& j, const OpcuaConfig& o) {
    j = {
        {"client",                       o.client},
        {"pipe",                         o.pipe},
        {"trigger_stop_ceiling_layers",  detail::opt_to_j(o.trigger_stop_ceiling_layers)},
        {"triggers",                     o.triggers},
        {"triggers_enabled",             detail::opt_to_j(o.triggers_enabled)},
    };
}

inline void from_json(const nlohmann::json& j, OpcuaConfig& o) {
    j.at("client").get_to(o.client);
    j.at("pipe").get_to(o.pipe);
    j.at("triggers").get_to(o.triggers);
    o.triggers_enabled = detail::j_to_opt<bool>(j, "triggers_enabled");
    o.trigger_stop_ceiling_layers = detail::j_to_opt<std::int64_t>(j, "trigger_stop_ceiling_layers");
}

// --- MachineConfig ---
// opcua: omitted from JSON entirely when absent (not serialised as null).

inline void to_json(nlohmann::json& j, const MachineConfig& m) {
    j = {
        {"machine",        m.machine},
        {"meta",           m.meta},
        {"optical_trains", m.optical_trains},
    };
    if (m.opcua.has_value())
        j["opcua"] = *m.opcua;
}

inline void from_json(const nlohmann::json& j, MachineConfig& m) {
    j.at("meta").get_to(m.meta);
    j.at("machine").get_to(m.machine);
    j.at("optical_trains").get_to(m.optical_trains);
    if (j.contains("opcua") && !j.at("opcua").is_null())
        m.opcua = j.at("opcua").get<OpcuaConfig>();
    else
        m.opcua = std::nullopt;
}

} // namespace machine_config
