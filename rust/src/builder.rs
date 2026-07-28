// Phase 3.7 — MockConfigBuilder: generates synthetic .h5 configs for testing.
// Mirrors Python's MockConfigBuilder (python/src/machine_config/builder.py).
// Values are deterministic/fixed so tests are reproducible across platforms.
// Gaussian correction grids are non-zero so correction-data tests have real
// values to assert against.

use std::path::Path;

use ndarray::Array3;

use crate::error::Result;
use crate::models::*;
use crate::writer::MachineConfigWriter;

const SCHEMA_VERSION: &str = "v1";
const MOCK_EXPORT_DATE: &str = "2026-01-01T00:00:00.000Z";

/// Generates a structurally valid synthetic machine-config `.h5` file.
/// Optical trains have a ClearBox with a non-zero Gaussian correction grid
/// and a placeholder `scan_field_correction_file` dataset.
pub struct MockConfigBuilder {
    pub laser_count: usize,
    pub build_plate_x: f64,
    pub build_plate_y: f64,
    pub build_plate_z: f64,
    pub include_clearbox: bool,
    pub machine_name: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
}

impl MockConfigBuilder {
    /// Create a builder for a `laser_count`-laser config with default values.
    pub fn new(laser_count: usize) -> Self {
        Self {
            laser_count,
            build_plate_x: 250.0,
            build_plate_y: 250.0,
            build_plate_z: 20.0,
            include_clearbox: true,
            machine_name: "MockMachine".to_owned(),
            manufacturer: "MockCo".to_owned(),
            model: "MockMIDI+".to_owned(),
            serial_number: "MOCK-001".to_owned(),
        }
    }

    /// Build the in-memory `MachineConfig` without writing to disk.
    pub fn build(&self) -> MachineConfig {
        let meta = MachineConfigMeta {
            schema_version: SCHEMA_VERSION.to_owned(),
            machine_name: self.machine_name.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            serial_number: self.serial_number.clone(),
            file_version: "1.0".to_owned(),
            export_date: MOCK_EXPORT_DATE.to_owned(),
            configuration_hash: "0".repeat(64),
            extra: Default::default(),
        };

        let machine = Machine {
            id: Some("00000000-0000-0000-0000-000000000001".to_owned()),
            machine_name: self.machine_name.clone(),
            manufacturer: self.manufacturer.clone(),
            model: self.model.clone(),
            serial_number: self.serial_number.clone(),
            build_plate_x: Some(self.build_plate_x),
            build_plate_x_unit: Some("mm".to_owned()),
            build_plate_y: Some(self.build_plate_y),
            build_plate_y_unit: Some("mm".to_owned()),
            build_plate_z: Some(self.build_plate_z),
            build_plate_z_unit: Some("mm".to_owned()),
            build_plate_radius: None,
            build_plate_radius_unit: None,
            gas_flow_direction: Some("Y+".to_owned()),
            recoat_direction: Some("X+".to_owned()),
        };

        let optical_trains = (0..self.laser_count)
            .map(|i| self.build_train(i))
            .collect();

        MachineConfig {
            meta,
            machine,
            optical_trains,
            opcua: None,
        }
    }

    /// Build and write the config to `path`.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        MachineConfigWriter::new(&self.build()).write(path)
    }

    // -----------------------------------------------------------------------
    // Private
    // -----------------------------------------------------------------------

    fn build_train(&self, index: usize) -> OpticalTrain {
        let sign = if index % 2 == 0 { -1.0_f64 } else { 1.0_f64 };

        let scanner = Scanner {
            manufacturer: "MockCo".to_owned(),
            model: "MockScan".to_owned(),
            serial_number: format!("MOCK-SC-{:02}", index + 1),
            working_distance: Some(670.0),
            working_distance_unit: Some("mm".to_owned()),
            scan_field_x: Some(600.0),
            scan_field_x_unit: Some("mm".to_owned()),
            scan_field_y: Some(600.0),
            scan_field_y_unit: Some("mm".to_owned()),
            scan_field_z: Some(76.5),
            scan_field_z_unit: Some("mm".to_owned()),
            scan_head_offset_x: Some(sign * 87.5),
            scan_head_offset_x_unit: Some("mm".to_owned()),
            scan_head_offset_y: Some(sign * -23.5),
            scan_head_offset_y_unit: Some("mm".to_owned()),
            scan_head_offset_z: Some(-1.0),
            scan_head_offset_z_unit: Some("mm".to_owned()),
            scan_head_rotation: Some(if index % 2 == 0 { 0.0 } else { 180.0 }),
            scan_head_rotation_unit: Some("degrees".to_owned()),
            axis_configuration: Some("3D".to_owned()),
            x_axis: mock_axis(),
            y_axis: mock_axis(),
            z_axis: Some(mock_axis()),
            focus: None,
        };

        let light_source = LightSource {
            manufacturer: "MockLaser".to_owned(),
            model: "MockFiber-1070".to_owned(),
            serial_number: format!("MOCK-LS-{:02}", index + 1),
            wavelength: Some(1070.0),
            wavelength_unit: Some("nm".to_owned()),
            power_max_nominal: Some(1000.0),
            power_max_nominal_unit: Some("W".to_owned()),
            power_max_actual: Some(1020.0),
            power_max_actual_unit: Some("W".to_owned()),
            power_min_actual: Some(100.0),
            power_min_actual_unit: Some("W".to_owned()),
            power_min_nominal: None,
            power_min_nominal_unit: Some("W".to_owned()),
            power_bit_resolution: None,
            power_bit_resolution_unit: Some("bits".to_owned()),
            watts_to_volts_algorithm: Some("LINEAR".to_owned()),
            watts_to_volts_params: Some("[1,100,10,1000]".to_owned()),
        };

        let collimator = Collimator {
            manufacturer: "MockOptics".to_owned(),
            model: "D50_F120".to_owned(),
            serial_number: format!("MOCK-COL-{:02}", index + 1),
            focal_length: Some(120.0),
            focal_length_unit: Some("mm".to_owned()),
        };

        let scanner_card = ScannerCard {
            manufacturer: "Raylase".to_owned(),
            model: "SP-ICE-3".to_owned(),
            serial_number: format!("MOCK-SC-CARD-{:02}", index + 1),
            communication_protocol: Some("SL2-100".to_owned()),
            sample_period: Some(10.0),
            sample_period_unit: Some("μs".to_owned()),
        };

        OpticalTrain {
            train_id: format!("Optical_Train_{:02}", index + 1),
            id: None,
            beam_profile_type: None,
            beam_waist_definition: Some("knife-edge".to_owned()),
            beam_waist_major: Some(67.0),
            beam_waist_major_unit: Some("μm".to_owned()),
            beam_waist_minor: Some(68.0),
            beam_waist_minor_unit: Some("μm".to_owned()),
            beam_waist_offset_z: Some(0.5),
            beam_waist_offset_z_unit: Some("mm".to_owned()),
            build_plane_offset_major: Some(0.2),
            build_plane_offset_major_unit: Some("mm".to_owned()),
            build_plane_offset_minor: Some(0.7),
            build_plane_offset_minor_unit: Some("mm".to_owned()),
            collimator_focal_length: Some(120.0),
            collimator_focal_length_unit: Some("mm".to_owned()),
            m2_major: Some(1.05),
            m2_minor: Some(1.08),
            major_axis_angle: Some(0.0),
            major_axis_angle_unit: Some("degrees".to_owned()),
            rayleigh_length_major: Some(3.1),
            rayleigh_length_major_unit: Some("mm".to_owned()),
            rayleigh_length_minor: Some(3.2),
            rayleigh_length_minor_unit: Some("mm".to_owned()),
            scanner_number: None,
            thermal_lensing_passed: Some(false),
            thermal_lensing_focal_plane_shift: Some(1.0),
            thermal_lensing_focal_plane_shift_unit: Some("mm".to_owned()),
            thermal_lensing_threshold: Some(0.75),
            thermal_lensing_threshold_unit: Some("mm".to_owned()),
            scanner,
            light_source,
            collimator,
            scanner_card,
            clearbox: if self.include_clearbox { Some(mock_clearbox(index)) } else { None },
            scan_field_correction_file: if self.include_clearbox { Some(mock_sfcf(index)) } else { None },
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn mock_axis() -> AxisConfig {
    AxisConfig {
        actual_bit_resolution: Some(20),
        actual_bit_resolution_unit: Some("bits".to_owned()),
        commanded_bit_resolution: Some(20),
        commanded_bit_resolution_unit: Some("bits".to_owned()),
        control_type: None,
        range_of_motion: None,
        range_of_motion_unit: Some("mm".to_owned()),
        smoothing_kernel: Some("GAUSSIAN".to_owned()),
        smoothing_parameters: Some(60.0),
        tuning_parameters: None,
        tuning_type: None,
    }
}

fn mock_clearbox(index: usize) -> ClearBox {
    let grid = gaussian_correction_grid();
    let mut inv = grid.clone();
    inv.mapv_inplace(|v| v * 0.9);
    ClearBox {
        ip_address: format!("192.168.1.{}", 10 + index),
        serial_number: Some(format!("{:03}", index + 1)),
        data_port: Some(5001),
        server_port: Some(20101),
        actual_timing_offset: Some(-8),
        commanded_timing_offset: Some(50),
        correction_data: Some(array3_to_nested(&grid)),
        inverse_correction_data: Some(array3_to_nested(&inv)),
        manufacturer: None,
        model: None,
        output_path: Some("/recordings/".to_owned()),
        selected_camera: Some("Default".to_owned()),
        custom_video_format: Some("MP4".to_owned()),
        video_output: Some("HDMI".to_owned()),
        show_console: Some(false),
        software_trigger_delay: Some(3000),
        volts_to_watts_algorithm: Some("LINEAR".to_owned()),
        volts_to_watts_params: Some("50.0,100.0".to_owned()),
        correction_grid_domain_shape: None,
        inverse_grid_domain_shape: None,
    }
}

fn mock_sfcf(index: usize) -> ScanFieldCorrectionFile {
    ScanFieldCorrectionFile {
        document_name: format!("mock_laser_{}.fc3", index + 1),
        document_id: format!("00000000-0000-0000-0000-{:012}", index + 1),
        file_size: 1024,
        valid_as_of_date: MOCK_EXPORT_DATE.to_owned(),
        document_created_at: None,
        document_type: Some("Scan Field Correction File".to_owned()),
        original_uri: None,
        raw_bytes: None,
    }
}

/// Smooth 2-D Gaussian warp pattern, shape (257, 257, 2).
/// Non-zero by design — zeros would hide correction-application bugs.
fn gaussian_correction_grid() -> Array3<f64> {
    let n = 257_usize;
    let mut grid = Array3::<f64>::zeros((n, n, 2));
    for i in 0..n {
        let x = -1.0_f64 + 2.0 * i as f64 / (n - 1) as f64;
        for j in 0..n {
            let y = -1.0_f64 + 2.0 * j as f64 / (n - 1) as f64;
            let warp = 2.0_f64 * (-(x * x + y * y) / 0.5_f64).exp();
            grid[[i, j, 0]] = warp;
            grid[[i, j, 1]] = warp * 0.8;
        }
    }
    grid
}

/// Convert a `(D0, D1, D2)` array to a nested `Vec` with NaN mapped to `None`.
fn array3_to_nested(arr: &Array3<f64>) -> Vec<Vec<Vec<Option<f64>>>> {
    let shape = arr.shape();
    (0..shape[0])
        .map(|i| {
            (0..shape[1])
                .map(|j| {
                    (0..shape[2])
                        .map(|k| {
                            let v = arr[[i, j, k]];
                            if v.is_nan() { None } else { Some(v) }
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::MachineConfigReader;

    #[test]
    fn builder_1_laser_roundtrip() {
        let tmp = tempfile();
        MockConfigBuilder::new(1).save(&tmp).unwrap();
        let config = MachineConfigReader::open(&tmp).unwrap().parse().unwrap();
        assert_eq!(config.optical_trains.len(), 1);
        assert_eq!(config.meta.machine_name, "MockMachine");
    }

    #[test]
    fn builder_2_laser_roundtrip() {
        let tmp = tempfile();
        MockConfigBuilder::new(2).save(&tmp).unwrap();
        let config = MachineConfigReader::open(&tmp).unwrap().parse().unwrap();
        assert_eq!(config.optical_trains.len(), 2);
    }

    #[test]
    fn builder_plate_dimensions() {
        let tmp = tempfile();
        MockConfigBuilder::new(1).save(&tmp).unwrap();
        let config = MachineConfigReader::open(&tmp).unwrap().parse().unwrap();
        assert_eq!(config.machine.build_plate_x, Some(250.0));
        assert_eq!(config.machine.build_plate_y, Some(250.0));
        assert_eq!(config.machine.build_plate_z, Some(20.0));
    }

    #[test]
    fn builder_correction_grid_shape() {
        let tmp = tempfile();
        MockConfigBuilder::new(1).save(&tmp).unwrap();
        let cd = MachineConfigReader::open(&tmp).unwrap().get_correction_data(0).unwrap();
        assert_eq!(cd.shape, [257, 257, 2]);
    }

    #[test]
    fn builder_correction_grid_nonzero() {
        let tmp = tempfile();
        MockConfigBuilder::new(1).save(&tmp).unwrap();
        let cd = MachineConfigReader::open(&tmp).unwrap().get_correction_data(0).unwrap();
        // Centre cell (128, 128, 0) must be the Gaussian peak (warp ≈ 2.0 for r=0).
        // Row-major index: i * d1 * d2 + j * d2 + k  where shape = [257, 257, 2].
        let centre = cd.data[128 * 257 * 2 + 128 * 2];
        assert!(centre > 1.9 && centre < 2.1, "centre={centre}");
        assert!(cd.data.iter().any(|v| v.abs() > 0.0), "all zeros");
    }

    #[test]
    fn builder_no_clearbox() {
        let tmp = tempfile();
        let mut b = MockConfigBuilder::new(1);
        b.include_clearbox = false;
        b.save(&tmp).unwrap();
        let config = MachineConfigReader::open(&tmp).unwrap().parse().unwrap();
        assert!(config.optical_trains[0].clearbox.is_none());
    }

    // Minimal temp-file helper for inline tests — avoids a `tempfile` crate
    // dependency in the lib target (it is a dev-dependency already from
    // Cargo.toml, but we can't use it in `#[cfg(test)]` inside lib without
    // adding it to regular deps, so we use a deterministic path in the OS
    // temp dir instead).
    fn tempfile() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        // Each call gets a unique name via a thread-local counter.
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        p.push(format!("mc_builder_test_{}.h5", COUNTER.fetch_add(1, Ordering::Relaxed)));
        p
    }
}
