#pragma once
// Machine Config Library — MockConfigBuilder (§4.20).
// Mirrors Python MockConfigBuilder and Rust MockConfigBuilder field-for-field.

#include "machine_config/models.hpp"
#include "machine_config/power_characterization.hpp"
#include "machine_config/writer.hpp"

#include <cmath>
#include <filesystem>
#include <string>

namespace machine_config {

// ---------------------------------------------------------------------------
// Correction grid helper
// ---------------------------------------------------------------------------

// 257×257×2 Gaussian correction grid (no NaN cells).
// channel 0 = warp, channel 1 = warp×0.8; scale=0.95 gives the inverse variant.
inline Grid3D makeGaussianGrid(double scale = 1.0) {
    constexpr size_t N = 257, C = 2;
    Grid3D grid(N, std::vector<std::vector<GridCell>>(N, std::vector<GridCell>(C)));
    for (size_t i = 0; i < N; ++i) {
        double x = -1.0 + 2.0 * static_cast<double>(i) / (N - 1.0);
        for (size_t j = 0; j < N; ++j) {
            double y    = -1.0 + 2.0 * static_cast<double>(j) / (N - 1.0);
            double warp = 2.0 * std::exp(-(x * x + y * y) / 0.5);
            grid[i][j][0] = GridCell{scale * warp};
            grid[i][j][1] = GridCell{scale * warp * 0.8};
        }
    }
    return grid;
}

// ---------------------------------------------------------------------------
// MockConfigBuilder
// ---------------------------------------------------------------------------

class MockConfigBuilder {
public:
    size_t laser_count   = 2;
    double build_plate_x = 250.0;
    double build_plate_y = 250.0;
    double build_plate_z = 20.0;
    bool   include_clearbox = true;
    std::string machine_name  = "MockMachine";
    std::string manufacturer  = "MockCo";
    std::string model         = "MockMIDI+";
    std::string serial_number = "MOCK-001";

    MachineConfig build() const {
        MachineConfig cfg;

        cfg.meta.schema_version     = "v1";
        cfg.meta.machine_name       = machine_name;
        cfg.meta.manufacturer       = manufacturer;
        cfg.meta.model              = model;
        cfg.meta.serial_number      = serial_number;
        cfg.meta.file_version       = "1.0";
        cfg.meta.export_date        = "2026-01-01T00:00:00.000Z";
        cfg.meta.configuration_hash = std::string(64, '0');
        cfg.meta.extra              = nlohmann::json::object();

        cfg.machine.machine_name  = machine_name;
        cfg.machine.manufacturer  = manufacturer;
        cfg.machine.model         = model;
        cfg.machine.serial_number = serial_number;
        cfg.machine.build_plate_x = build_plate_x;
        cfg.machine.build_plate_x_unit = "mm";
        cfg.machine.build_plate_y = build_plate_y;
        cfg.machine.build_plate_y_unit = "mm";
        cfg.machine.build_plate_z = build_plate_z;
        cfg.machine.build_plate_z_unit = "mm";
        cfg.machine.gas_flow_direction = "Y+";
        cfg.machine.recoat_direction   = "X+";
        cfg.machine.recoater_blade_type = "Standard";

        for (size_t i = 0; i < laser_count; ++i)
            cfg.optical_trains.push_back(mockTrain(i));

        return cfg;
    }

    void save(std::filesystem::path path) const {
        MachineConfigWriter{build()}.write(std::move(path));
    }

private:
    static AxisConfig mockAxis() {
        AxisConfig a;
        a.actual_bit_resolution      = 20;
        a.actual_bit_resolution_unit = "bits";
        a.commanded_bit_resolution      = 20;
        a.commanded_bit_resolution_unit = "bits";
        a.range_of_motion_unit = "mm";
        a.smoothing_kernel     = "GAUSSIAN";
        a.smoothing_parameters = 60.0;
        return a;
    }

    static ClearBox mockClearbox(size_t index) {
        ClearBox cb;
        cb.ip_address = "192.168.1." + std::to_string(10 + static_cast<int>(index));
        std::string sn = std::to_string(index + 1);
        cb.serial_number = std::string(3 - sn.size(), '0') + sn; // zero-padded, e.g. "001"
        cb.data_port                = 5001;
        cb.server_port              = 20101;
        cb.actual_timing_offset     = -8;
        cb.commanded_timing_offset  = 50;
        cb.output_path              = "/recordings/";
        cb.selected_camera          = "Default";
        cb.custom_video_format      = "MP4";
        cb.video_output             = "HDMI";
        cb.show_console             = false;
        cb.software_trigger_delay   = 3000;
        cb.power_characterization   = forwardPowerCharacterizationCoefficients("LINEAR", "50.0,100.0");
        cb.correction_data         = makeGaussianGrid(1.0);
        cb.inverse_correction_data = makeGaussianGrid(0.95);
        return cb;
    }

    static ScanFieldCorrectionFile mockSfcf(size_t index) {
        ScanFieldCorrectionFile s;
        s.document_name    = "mock_laser_" + std::to_string(index + 1) + ".fc3";
        s.document_id      = "00000000-0000-0000-0000-000000000000";
        s.file_size        = 1024;
        s.valid_as_of_date = "2026-01-01T00:00:00.000Z";
        s.document_type    = "Scan Field Correction File";
        return s;
    }

    OpticalTrain mockTrain(size_t index) const {
        int sign = (index % 2 == 0) ? -1 : 1;

        Scanner sc;
        sc.manufacturer          = "MockCo";
        sc.model                 = "MockScan";
        sc.serial_number         = "MOCK-SC-" + zeroPad(index + 1, 2);
        sc.working_distance      = 670.0;
        sc.working_distance_unit = "mm";
        sc.scan_field_x          = 600.0;
        sc.scan_field_x_unit     = "mm";
        sc.scan_field_y          = 600.0;
        sc.scan_field_y_unit     = "mm";
        sc.scan_field_z          = 76.5;
        sc.scan_field_z_unit     = "mm";
        sc.scan_head_offset_x      = sign * 87.5;
        sc.scan_head_offset_x_unit = "mm";
        sc.scan_head_offset_y      = sign * -23.5;
        sc.scan_head_offset_y_unit = "mm";
        sc.scan_head_offset_z      = -1.0;
        sc.scan_head_offset_z_unit = "mm";
        sc.scan_head_rotation      = (index % 2 == 0) ? 0.0 : 180.0;
        sc.scan_head_rotation_unit = "degrees";
        sc.axis_configuration      = "3D";
        sc.x_axis = mockAxis();
        sc.y_axis = mockAxis();
        sc.z_axis = mockAxis();

        LightSource ls;
        ls.manufacturer           = "MockLaser";
        ls.model                  = "MockFiber-1070";
        ls.serial_number          = "MOCK-LS-" + zeroPad(index + 1, 2);
        ls.wavelength             = 1070.0;
        ls.wavelength_unit        = "nm";
        ls.power_max_nominal      = 1000.0;
        ls.power_max_nominal_unit = "W";
        ls.power_max_actual       = 1020.0;
        ls.power_max_actual_unit  = "W";
        ls.power_min_actual       = 100.0;
        ls.power_min_actual_unit  = "W";
        ls.power_characterization = forwardPowerCharacterizationPoints("LINEAR", "[1,100,10,1000]");

        Collimator col;
        col.manufacturer      = "MockOptics";
        col.model             = "D50_F120";
        col.serial_number     = "MOCK-COL-" + zeroPad(index + 1, 2);
        col.focal_length      = 120.0;
        col.focal_length_unit = "mm";

        ScannerCard sk;
        sk.manufacturer           = "Raylase";
        sk.model                  = "SP-ICE-3";
        sk.serial_number          = "MOCK-SC-CARD-" + zeroPad(index + 1, 2);
        sk.communication_protocol = "SL2-100";
        sk.sample_period          = 10.0;
        sk.sample_period_unit     = "\xce\xbcs"; // μs (UTF-8)

        OptionalComponents oc;
        if (include_clearbox)
            oc.clearbox = mockClearbox(index);

        OpticalTrain t;
        t.train_id                               = "Optical_Train_" + zeroPad(index + 1, 2);
        t.beam_waist_definition                  = "knife-edge";
        t.beam_waist_major                       = 67.0;
        t.beam_waist_major_unit                  = "\xce\xbcm"; // μm (UTF-8)
        t.beam_waist_minor                       = 68.0;
        t.beam_waist_minor_unit                  = "\xce\xbcm";
        t.beam_waist_offset_z                    = 0.5;
        t.beam_waist_offset_z_unit               = "mm";
        t.build_plane_offset_major               = 0.2;
        t.build_plane_offset_major_unit          = "mm";
        t.build_plane_offset_minor               = 0.7;
        t.build_plane_offset_minor_unit          = "mm";
        t.collimator_focal_length                = 120.0;
        t.collimator_focal_length_unit           = "mm";
        t.m2_major                               = 1.05;
        t.m2_minor                               = 1.08;
        t.major_axis_angle                       = 0.0;
        t.major_axis_angle_unit                  = "degrees";
        t.rayleigh_length_major                  = 3.1;
        t.rayleigh_length_major_unit             = "mm";
        t.rayleigh_length_minor                  = 3.2;
        t.rayleigh_length_minor_unit             = "mm";
        t.thermal_lensing_passed                 = false;
        t.thermal_lensing_focal_plane_shift      = 1.0;
        t.thermal_lensing_focal_plane_shift_unit = "mm";
        t.thermal_lensing_threshold              = 0.75;
        t.thermal_lensing_threshold_unit         = "mm";
        t.scanner                                = std::move(sc);
        t.light_source                           = std::move(ls);
        t.collimator                             = std::move(col);
        t.scanner_card                           = std::move(sk);
        t.optional_components                    = std::move(oc);
        if (include_clearbox)
            t.scan_field_correction_file = mockSfcf(index);
        return t;
    }

    static std::string zeroPad(size_t n, int width) {
        std::string s = std::to_string(n);
        if (static_cast<int>(s.size()) < width)
            s = std::string(width - static_cast<int>(s.size()), '0') + s;
        return s;
    }
};

} // namespace machine_config
