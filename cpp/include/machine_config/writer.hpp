#pragma once
// Machine Config Library — HDF5 writer (header-only, §4.14).
// Exact inverse of reader.hpp; every attribute name and type mirrors the reader.
// None/nullopt → "" string; bool → int64 0/1.

#include "machine_config/models.hpp"

#include <highfive/H5File.hpp>

#include <filesystem>
#include <limits>
#include <string>

namespace machine_config {

// ---------------------------------------------------------------------------
// Attribute-writing helpers (inverses of reader.hpp read helpers)
// ---------------------------------------------------------------------------

// VarLen UTF-8 string attribute — matches Python h5py default string type.
template <typename Loc>
inline void ws(Loc& loc, const std::string& key, const std::string& val) {
    loc.template createAttribute<std::string>(key, HighFive::DataSpace::Scalar()).write(val);
}

// float64 attribute; nullopt → "".
template <typename Loc>
inline void wf(Loc& loc, const std::string& key, std::optional<double> val) {
    if (val)
        loc.template createAttribute<double>(key, HighFive::DataSpace::Scalar()).write(*val);
    else
        ws(loc, key, "");
}

// int64 attribute; nullopt → "".
template <typename Loc>
inline void wi(Loc& loc, const std::string& key, std::optional<int64_t> val) {
    if (val)
        loc.template createAttribute<int64_t>(key, HighFive::DataSpace::Scalar()).write(*val);
    else
        ws(loc, key, "");
}

// bool → int64 0/1; nullopt → "".
template <typename Loc>
inline void wb(Loc& loc, const std::string& key, std::optional<bool> val) {
    if (val) {
        int64_t v = *val ? 1LL : 0LL;
        loc.template createAttribute<int64_t>(key, HighFive::DataSpace::Scalar()).write(v);
    } else {
        ws(loc, key, "");
    }
}

// Write extra attrs back preserving type (string / int / float).
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
// Correction grid helper
// ---------------------------------------------------------------------------

// Grid3D → flat row-major double buffer.  None cells become NaN.
// Absent Grid3D becomes a zero-filled default (257,257,2) array.
inline std::vector<double> gridToFlat(const std::optional<Grid3D>& grid,
                                       std::array<size_t, 3>& shape_out) {
    constexpr size_t D0 = 257, D1 = 257, D2 = 2;
    if (!grid || grid->empty()) {
        shape_out = {D0, D1, D2};
        return std::vector<double>(D0 * D1 * D2, 0.0);
    }
    const auto& g = *grid;
    size_t d0 = g.size();
    size_t d1 = d0 > 0 ? g[0].size() : 0;
    size_t d2 = d1 > 0 ? g[0][0].size() : 0;
    if (d0 == 0 || d1 == 0 || d2 == 0) {
        shape_out = {D0, D1, D2};
        return std::vector<double>(D0 * D1 * D2, 0.0);
    }
    shape_out = {d0, d1, d2};
    const double nan = std::numeric_limits<double>::quiet_NaN();
    std::vector<double> flat(d0 * d1 * d2, nan);
    for (size_t i = 0; i < d0; ++i)
        for (size_t j = 0; j < d1; ++j)
            for (size_t k = 0; k < d2; ++k)
                flat[i * d1 * d2 + j * d2 + k] = g[i][j][k].value_or(nan);
    return flat;
}

// ---------------------------------------------------------------------------
// MachineConfigWriter
// ---------------------------------------------------------------------------

class MachineConfigWriter {
public:
    explicit MachineConfigWriter(const MachineConfig& cfg) : cfg_(cfg) {}

    void write(std::filesystem::path path) const {
        HighFive::File f(path.string(),
            HighFive::File::ReadWrite | HighFive::File::Create | HighFive::File::Truncate);
        writeRootAttrs(f);
        auto mgrp = f.createGroup("Machine");
        writeMachineAttrs(mgrp);
        auto ogrp = mgrp.createGroup("Optical_Trains");
        for (size_t i = 0; i < cfg_.optical_trains.size(); ++i) {
            auto n = std::to_string(i + 1);
            if (n.size() < 2) n = "0" + n;
            auto tg = ogrp.createGroup("Optical_Train_" + n);
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
        writeExtra(f, m.extra);
    }

    void writeMachineAttrs(HighFive::Group& grp) const {
        const auto& ma = cfg_.machine;
        ws(grp, "ID",                            ma.id.value_or(""));
        ws(grp, "Machine_Name",                  ma.machine_name);
        ws(grp, "Manufacturer",                  ma.manufacturer);
        ws(grp, "Model",                         ma.model);
        ws(grp, "Serial_Number",                 ma.serial_number);
        wf(grp, "Build_Plate_X_Dimension",       ma.build_plate_x);
        ws(grp, "Build_Plate_X_Dimension_unit",  ma.build_plate_x_unit.value_or("mm"));
        wf(grp, "Build_Plate_Y_Dimension",       ma.build_plate_y);
        ws(grp, "Build_Plate_Y_Dimension_unit",  ma.build_plate_y_unit.value_or("mm"));
        wf(grp, "Build_Plate_Z_Dimension",       ma.build_plate_z);
        ws(grp, "Build_Plate_Z_Dimension_unit",  ma.build_plate_z_unit.value_or("mm"));
        wf(grp, "Build_Plate_Corner_Radius",     ma.build_plate_radius);
        ws(grp, "Build_Plate_Corner_Radius_unit",ma.build_plate_radius_unit.value_or("mm"));
        ws(grp, "Gas_Flow_Direction",            ma.gas_flow_direction.value_or(""));
        ws(grp, "Recoat_Direction",              ma.recoat_direction.value_or(""));
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
        wf(grp, "Working_Distance",        s.working_distance);
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
        // Real HDF5 files store Power_Bit_Resolution as a string (see §3.11).
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
    }

    void writeCorrectionDataset(HighFive::Group& grp, const std::string& name,
                                  const std::optional<Grid3D>& grid) const {
        std::array<size_t, 3> shape{};
        auto flat = gridToFlat(grid, shape);
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
            "scan_field_correction_file", HighFive::DataSpace({data.size()}));
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
        writeExtra(cg, c.extra);

        auto pg = og.createGroup("Pipe");
        const auto& p = opcua.pipe;
        pg.template createAttribute<int64_t>("Pipe_Enabled", HighFive::DataSpace::Scalar()).write(static_cast<int64_t>(p.pipe_enabled));
        pg.template createAttribute<int64_t>("Buffer_Size",  HighFive::DataSpace::Scalar()).write(p.buffer_size);
        writeExtra(pg, p.extra);

        auto tg = og.createGroup("Triggers");
        if (opcua.triggers_enabled) {
            // Stored as float64 to match original HDF5 file format (see §3.10).
            double te = *opcua.triggers_enabled ? 1.0 : 0.0;
            tg.template createAttribute<double>("Triggers_Enabled", HighFive::DataSpace::Scalar()).write(te);
        }
        for (const auto& [name, trigger] : opcua.triggers) {
            auto trg = tg.createGroup(name);
            ws(trg, "ID",          trigger.id.value_or(""));
            ws(trg, "Signal",      trigger.signal.value_or(""));
            ws(trg, "Subsystem",   trigger.subsystem.value_or(""));
            wb(trg, "Rule_Enabled",trigger.rule_enabled);
            ws(trg, "Start_Value", trigger.start_value.value_or(""));
            ws(trg, "Stop_Value",  trigger.stop_value.value_or(""));
            writeExtra(trg, trigger.extra);
        }
    }
};

} // namespace machine_config
