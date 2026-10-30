#pragma once
// File_Version 1.1 HDF5 writer — on-disk layout, attribute names, and
// casting. Public MachineConfigWriter dispatches here after reading
// File_Version from the model.
//
// Deliberately independent of any other version's own writer header — see
// the isolation rule enforced by
// cpp/tests/test_version_adapter_isolation.cpp and the matching note in
// this file's sibling hdf5.hpp.

#include "machine_config/adapters.hpp"
#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_1/compound_types.hpp"
#include "machine_config/capabilities/v1_1/layout.hpp"
#include "machine_config/extensions_hdf5.hpp"

#include <highfive/H5File.hpp>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <map>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace machine_config::capabilities::v1_1 {

// ---------------------------------------------------------------------------
// Attribute-writing helpers (inverses of hdf5.hpp's read helpers) —
// independent copies; see this directory's isolation note.
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

// Writes an int64 attribute (1) only when val is true; writes nothing at
// all otherwise — matches Scanner's four invert_* fields' existing
// round-trip convention.
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
// Hdf5WriterV1_1
// ---------------------------------------------------------------------------

class Hdf5WriterV1_1 : public WriterAdapter {
public:
    explicit Hdf5WriterV1_1(const MachineConfig& cfg) : cfg_(cfg) {}

    // ClearBox is an ordinary extension tree here: the shared Output_Path/Software_Trigger_Delay
    // live on its root and one subgroup per train beneath it, so every extension is written the
    // same way.
    void write(std::filesystem::path path) const override {
        validateExtensions();
        HighFive::File f(path.string(),
            HighFive::File::ReadWrite | HighFive::File::Create | HighFive::File::Truncate);
        writeRootAttrs(f);
        auto mgrp = f.createGroup(ROOT_MACHINE);
        writeMachineAttrs(mgrp);
        auto ogrp = mgrp.createGroup("Optical_Trains");

        for (size_t i = 0; i < cfg_.optical_trains.size(); ++i) {
            std::string tid = trainId(i);
            const auto& train = cfg_.optical_trains[i];
            auto tg = ogrp.createGroup(tid);
            writeTrainAttrs(tg, train);
            auto sg = tg.createGroup(GROUP_SCANNER);       writeScanner(sg, train.scanner);
            auto lg = tg.createGroup(GROUP_LIGHT_SOURCE);  writeLightSource(lg, train.light_source);
            auto cg = tg.createGroup(GROUP_COLLIMATOR);    writeCollimator(cg, train.collimator);
            auto kg = tg.createGroup(GROUP_SCANNER_CARD);  writeScannerCard(kg, train.scanner_card);
            if (train.scan_field_correction_file)
                writeSfcf(f, tid, *train.scan_field_correction_file);
        }

        if (!cfg_.extensions.empty()) {
            auto eg = f.exist(ROOT_EXTENSIONS) ? f.getGroup(ROOT_EXTENSIONS) : f.createGroup(ROOT_EXTENSIONS);
            for (const auto& [name, tree] : cfg_.extensions)
                extension::writeExtensionTree(eg.getId(), name, tree);
        }
    }

private:
    const MachineConfig& cfg_;

    // Runs before the output file is created, so a rejected config never leaves a partial file
    // behind.
    void validateExtensions() const {
        for (const auto& [name, tree] : cfg_.extensions) extension::validate(tree, name);
    }

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
        ws(grp, "Build_Plate_X_Dimension_unit",  ma.build_plate_x_unit.value_or(""));
        wf(grp, "Build_Plate_Y_Dimension",       ma.build_plate_y);
        ws(grp, "Build_Plate_Y_Dimension_unit",  ma.build_plate_y_unit.value_or(""));
        wf(grp, "Build_Plate_Z_Dimension",       ma.build_plate_z);
        ws(grp, "Build_Plate_Z_Dimension_unit",  ma.build_plate_z_unit.value_or(""));
        wf(grp, "Build_Plate_Corner_Radius",     ma.build_plate_radius);
        ws(grp, "Build_Plate_Corner_Radius_unit",ma.build_plate_radius_unit.value_or(""));
        ws(grp, "Gas_Flow_Direction",            ma.gas_flow_direction.value_or(""));
        ws(grp, "Recoat_Direction",              ma.recoat_direction.value_or(""));
        ws(grp, "Recoater_Blade_Type",           ma.recoater_blade_type.value_or(""));
    }

    void writeTrainAttrs(HighFive::Group& grp, const OpticalTrain& t) const {
        ws(grp, "ID",                                 t.id.value_or(""));
        ws(grp, "Beam_Profile_Type",                  t.beam_profile_type.value_or(""));
        ws(grp, "Beam_Waist_Definition",              t.beam_waist_definition.value_or(""));
        wf(grp, "Beam_Waist_Major",                   t.beam_waist_major);
        ws(grp, "Beam_Waist_Major_unit",              t.beam_waist_major_unit.value_or(""));
        wf(grp, "Beam_Waist_Minor",                   t.beam_waist_minor);
        ws(grp, "Beam_Waist_Minor_unit",              t.beam_waist_minor_unit.value_or(""));
        wf(grp, "Beam_Waist_Offset_Z",                t.beam_waist_offset_z);
        ws(grp, "Beam_Waist_Offset_Z_unit",           t.beam_waist_offset_z_unit.value_or(""));
        wf(grp, "Build_Plane_Offset_Major",           t.build_plane_offset_major);
        ws(grp, "Build_Plane_Offset_Major_unit",      t.build_plane_offset_major_unit.value_or(""));
        wf(grp, "Build_Plane_Offset_Minor",           t.build_plane_offset_minor);
        ws(grp, "Build_Plane_Offset_Minor_unit",      t.build_plane_offset_minor_unit.value_or(""));
        wf(grp, "Collimator_Focal_Length",            t.collimator_focal_length);
        ws(grp, "Collimator_Focal_Length_unit",       t.collimator_focal_length_unit.value_or(""));
        wf(grp, "M2_Major",                           t.m2_major);
        wf(grp, "M2_Minor",                           t.m2_minor);
        wf(grp, "Major_Axis_Angle",                   t.major_axis_angle);
        ws(grp, "Major_Axis_Angle_unit",              t.major_axis_angle_unit.value_or(""));
        wf(grp, "Rayleigh_Length_Major",              t.rayleigh_length_major);
        ws(grp, "Rayleigh_Length_Major_unit",         t.rayleigh_length_major_unit.value_or(""));
        wf(grp, "Rayleigh_Length_Minor",              t.rayleigh_length_minor);
        ws(grp, "Rayleigh_Length_Minor_unit",         t.rayleigh_length_minor_unit.value_or(""));
        ws(grp, "Scanner_Number",                     t.scanner_number.value_or(""));
        wb(grp, "Thermal_Lensing_Test_Passed",        t.thermal_lensing_passed);
        wf(grp, "Thermal_Lensing_Focal_Plane_Shift",       t.thermal_lensing_focal_plane_shift);
        ws(grp, "Thermal_Lensing_Focal_Plane_Shift_unit",  t.thermal_lensing_focal_plane_shift_unit.value_or(""));
        wf(grp, "Thermal_Lensing_Threshold",          t.thermal_lensing_threshold);
        ws(grp, "Thermal_Lensing_Threshold_unit",     t.thermal_lensing_threshold_unit.value_or(""));
    }

    // Change 5: Tuning_Parameters/Tuning_Type are never written in this
    // File_Version, regardless of what's on the model.
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
    }

    void writeScanner(HighFive::Group& grp, const Scanner& s) const {
        ws(grp, "Manufacturer",            s.manufacturer);
        ws(grp, "Model",                   s.model);
        ws(grp, "Serial_Number",           s.serial_number);
        wf(grp, "Working_Distance",        s.working_distance);
        ws(grp, "Working_Distance_unit",   s.working_distance_unit.value_or(""));
        wf(grp, "Scan_Field_Size_X",       s.scan_field_x);
        ws(grp, "Scan_Field_Size_X_unit",  s.scan_field_x_unit.value_or(""));
        wf(grp, "Scan_Field_Size_Y",       s.scan_field_y);
        ws(grp, "Scan_Field_Size_Y_unit",  s.scan_field_y_unit.value_or(""));
        wf(grp, "Scan_Field_Size_Z",       s.scan_field_z);
        ws(grp, "Scan_Field_Size_Z_unit",  s.scan_field_z_unit.value_or(""));
        wf(grp, "Scan_Head_Offset_X",      s.scan_head_offset_x);
        ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit.value_or(""));
        wf(grp, "Scan_Head_Offset_Y",      s.scan_head_offset_y);
        ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit.value_or(""));
        wf(grp, "Scan_Head_Offset_Z",      s.scan_head_offset_z);
        ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit.value_or(""));
        wf(grp, "Scan_Head_Rotation",      s.scan_head_rotation);
        ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit.value_or(""));
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

    // Change 3/4: schema-complete over omitted — always writes a complete,
    // present group with both datasets, even when *pc* is nullopt or its
    // vectors are empty. See the migration manifest's derivation rules.
    void writePowerCharacterization(HighFive::Group& grp, const std::optional<PowerCharacterization>& pc) const {
        ws(grp, "Algorithm_Type",         pc && pc->algorithm_type ? *pc->algorithm_type : std::string());
        ws(grp, "Algorithm_Equation",     pc && pc->algorithm_equation ? *pc->algorithm_equation : std::string());
        ws(grp, "Input_Type",             pc && pc->input_type ? *pc->input_type : std::string());
        ws(grp, "Units_Derived_Quantity", pc && pc->units_derived_quantity ? *pc->units_derived_quantity : std::string());

        const std::vector<EquationConstant> empty_constants;
        writeEquationConstantsDataset(grp, DS_DERIVATION_EQUATION_CONSTANTS,
            pc ? pc->derivation_equation_constants : empty_constants);

        const std::vector<CalibrationPoint> empty_points;
        writeCalibrationPointsDataset(grp, DS_CHARACTERIZATION_POINTS,
            pc ? pc->characterization_points : empty_points);
    }

    void writeLightSource(HighFive::Group& grp, const LightSource& ls) const {
        ws(grp, "Manufacturer",               ls.manufacturer);
        ws(grp, "Model",                      ls.model);
        ws(grp, "Serial_Number",              ls.serial_number);
        wf(grp, "Light_Wavelength",           ls.wavelength);
        ws(grp, "Light_Wavelength_unit",      ls.wavelength_unit.value_or(""));
        wf(grp, "Power_Max_Nominal",          ls.power_max_nominal);
        ws(grp, "Power_Max_Nominal_unit",     ls.power_max_nominal_unit.value_or(""));
        wf(grp, "Power_Max_Actual",           ls.power_max_actual);
        ws(grp, "Power_Max_Actual_unit",      ls.power_max_actual_unit.value_or(""));
        wf(grp, "Power_Min_Actual",           ls.power_min_actual);
        ws(grp, "Power_Min_Actual_unit",      ls.power_min_actual_unit.value_or(""));
        wf(grp, "Power_Min_Nominal",          ls.power_min_nominal);
        ws(grp, "Power_Min_Nominal_unit",     ls.power_min_nominal_unit.value_or(""));
        // Real HDF5 files store Power_Bit_Resolution as a string.
        std::string pbr = ls.power_bit_resolution
            ? nlohmann::json(*ls.power_bit_resolution).dump()
            : std::string{};
        ws(grp, "Power_Bit_Resolution",       pbr);
        ws(grp, "Power_Bit_Resolution_unit",  ls.power_bit_resolution_unit.value_or(""));
        // Change 4: Watts_To_Volts_Algorithm/Params are not written in this
        // File_Version — power_characterization is the only representation.
        auto pc = ls.power_characterization;
        auto pcGrp = grp.createGroup(GROUP_POWER_CHARACTERIZATION);
        writePowerCharacterization(pcGrp, pc);
    }

    void writeCollimator(HighFive::Group& grp, const Collimator& c) const {
        ws(grp, "Manufacturer",      c.manufacturer);
        ws(grp, "Model",             c.model);
        ws(grp, "Serial_Number",     c.serial_number);
        wf(grp, "Focal_Length",      c.focal_length);
        ws(grp, "Focal_Length_unit", c.focal_length_unit.value_or(""));
    }

    void writeScannerCard(HighFive::Group& grp, const ScannerCard& sc) const {
        ws(grp, "Manufacturer",            sc.manufacturer);
        ws(grp, "Model",                   sc.model);
        ws(grp, "Serial_Number",           sc.serial_number);
        ws(grp, "Communication_Protocol",  sc.communication_protocol.value_or(""));
        wf(grp, "Sample_Period",           sc.sample_period);
        ws(grp, "Sample_Period_unit",      sc.sample_period_unit.value_or(""));
    }

    // Derivation_Equation_Constants.name is a 64-byte fixed-length field. A
    // name whose UTF-8 encoding exceeds 64 bytes is rejected here, not
    // silently truncated on write.
    void writeEquationConstantsDataset(HighFive::Group& grp, const std::string& dsName,
                                        const std::vector<EquationConstant>& constants) const {
        std::vector<V1_1EquationConstantRow> rows;
        rows.reserve(constants.size());
        for (const auto& c : constants) {
            if (c.name.size() > V1_1EquationConstantMaxNameBytes) {
                throw std::runtime_error(
                    "Derivation_Equation_Constants name '" + c.name + "' is " +
                    std::to_string(c.name.size()) + " UTF-8 bytes, which does not fit in the " +
                    std::to_string(V1_1EquationConstantMaxNameBytes) +
                    "-byte fixed-length field (would otherwise be silently truncated on write).");
            }
            V1_1EquationConstantRow row{};
            std::memset(row.name, 0, sizeof(row.name));
            std::memcpy(row.name, c.name.data(), c.name.size());
            row.value = c.value;
            rows.push_back(row);
        }
        grp.createDataSet(dsName, rows);
    }

    void writeCalibrationPointsDataset(HighFive::Group& grp, const std::string& dsName,
                                        const std::vector<CalibrationPoint>& points) const {
        std::vector<V1_1CalibrationPointRow> rows;
        rows.reserve(points.size());
        for (const auto& p : points)
            rows.push_back({p.input_value, p.output_value});
        grp.createDataSet(dsName, rows);
    }

    void writeSfcf(HighFive::File& f, const std::string& tid, const ScanFieldCorrectionFile& sfcf) const {
        std::vector<uint8_t> data;
        if (sfcf.raw_bytes)
            data = *sfcf.raw_bytes;
        else
            data.resize(static_cast<size_t>(sfcf.file_size > 0 ? sfcf.file_size : 1), 0);
        auto ds = f.createDataSet<uint8_t>(
            scanFieldCorrectionFilePathById(tid), HighFive::DataSpace({data.size()}));
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

};

}  // namespace machine_config::capabilities::v1_1
