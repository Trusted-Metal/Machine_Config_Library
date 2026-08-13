//! Public HDF5 reader: peek File_Version, then dispatch to a version adapter.
//!
//! On-disk group paths and HDF5 attribute names live in the matching adapter.

use std::path::Path;

use crate::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use crate::error::{MachineConfigError, Result};
use crate::models::{CorrectionData, ExtraAttrs, MachineConfig};

pub use crate::capabilities::v1_0::hdf5::peek_file_version;

/// Reads a machine-config HDF5 file into a [`MachineConfig`].
///
/// Peeks root `File_Version` and dispatches to that version's adapter.
pub struct MachineConfigReader {
    backend: Hdf5AdapterV1_0,
}

impl MachineConfigReader {
    /// Opens the file, peeks `File_Version`, and selects the matching adapter.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let version = peek_file_version(path)?;
        match version.as_str() {
            "1.0" => Ok(Self {
                backend: Hdf5AdapterV1_0::open(path)?,
            }),
            other => Err(MachineConfigError::UnsupportedVersion(other.to_string())),
        }
    }

    pub fn parse(&self) -> Result<MachineConfig> {
        self.backend.parse()
    }

    pub fn parse_with_binary(&self) -> Result<MachineConfig> {
        self.backend.parse_with_binary()
    }

    pub fn get_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        self.backend.get_correction_data(train_index)
    }

    pub fn get_inverse_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        self.backend.get_inverse_correction_data(train_index)
    }

    pub fn get_scan_field_correction_bytes(&self, train_index: usize) -> Result<Vec<u8>> {
        self.backend.get_scan_field_correction_bytes(train_index)
    }

    pub fn get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs> {
        self.backend.get_raw_group(hdf5_path)
    }

    pub fn to_json(&self, pretty: bool, include_binary: bool) -> Result<String> {
        self.backend.to_json(pretty, include_binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");
    const REFERENCE_OPCUA: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5");
    const SYNTHETIC: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic_2laser.h5");

    #[test]
    fn open_rejects_nonexistent_path() {
        assert!(MachineConfigReader::open("does/not/exist.h5").is_err());
    }

    #[test]
    fn parse_reference_meta_and_machine() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        assert_eq!(config.meta.machine_name, "TM-LPBF-02: AconityMIDI+_OG");
        assert_eq!(config.meta.manufacturer, "Aconity3D");
        assert_eq!(config.meta.file_version, "1.0");
        assert_eq!(config.meta.configuration_hash.len(), 64);
        assert_eq!(config.meta.schema_version, "v1");
        assert_eq!(
            config.meta.extra.get("Description").and_then(|v| v.as_str()),
            Some("Machine Configuration Export from Service Observations")
        );

        assert_eq!(config.machine.build_plate_x, Some(250.0));
        assert_eq!(config.machine.build_plate_x_unit, Some("mm".to_string()));
        assert_eq!(config.machine.build_plate_z, Some(20.0));
        assert_eq!(config.machine.gas_flow_direction, Some("Y+".to_string()));
    }

    #[test]
    fn parse_reference_optical_trains() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        assert_eq!(config.optical_trains.len(), 2);

        let t1 = &config.optical_trains[0];
        assert_eq!(t1.train_id, "Optical_Train_01");
        assert_eq!(t1.scanner.working_distance, Some(670.0));
        assert_eq!(t1.scanner.working_distance_unit, Some("mm".to_string()));
        assert_eq!(t1.scanner.scan_head_offset_x, Some(-87.5));
        assert_eq!(t1.scanner.axis_configuration, Some("3D".to_string()));
        assert_eq!(t1.thermal_lensing_passed, Some(false));
        assert_eq!(t1.scanner.x_axis.smoothing_kernel, Some("GAUSSIAN".to_string()));
        assert!(t1.scanner.z_axis.is_some());
        assert!(t1.scanner.focus.is_none());
        assert_eq!(t1.scanner.x_axis.range_of_motion, None);

        let t2 = &config.optical_trains[1];
        assert_eq!(t2.train_id, "Optical_Train_02");
        assert_eq!(t2.thermal_lensing_passed, Some(true));
        assert_eq!(t2.scanner.scan_head_rotation, Some(180.0));
    }

    #[test]
    fn parse_reference_clearbox_and_sfcf_metadata_without_binary() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let t1 = &config.optical_trains[0];

        let cb = t1.optional_components.clearbox.as_ref().expect("clearbox present on reference fixture");
        assert!(!cb.ip_address.is_empty());
        assert!(cb.correction_data.is_none(), "parse() must not read binary correction grids");
        assert!(cb.inverse_correction_data.is_none());

        let sfcf = t1
            .scan_field_correction_file
            .as_ref()
            .expect("scan field correction file present");
        assert_eq!(sfcf.file_size, 1138799);
        assert!(sfcf.raw_bytes.is_none(), "parse() must not read raw .fc3 bytes");
    }

    #[test]
    fn parse_with_binary_populates_correction_data_and_raw_bytes() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse_with_binary().unwrap();
        let t1 = &config.optical_trains[0];

        let cb = t1.optional_components.clearbox.as_ref().unwrap();
        let data = cb.correction_data.as_ref().expect("correction_data populated");
        assert_eq!(data.len(), 257);
        assert_eq!(data[0].len(), 257);
        assert_eq!(data[0][0].len(), 2);

        let sfcf = t1.scan_field_correction_file.as_ref().unwrap();
        let raw = sfcf.raw_bytes.as_ref().expect("raw_bytes populated");
        assert_eq!(raw.len(), sfcf.file_size as usize);
    }

    #[test]
    fn get_correction_data_shape_and_nan_present() {
        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let cd = reader.get_correction_data(0).unwrap();
        assert_eq!(cd.shape, [257, 257, 2]);
        assert!(cd.data.iter().any(|v| v.is_nan()));
    }

    #[test]
    fn get_scan_field_correction_bytes_matches_file_size() {
        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let bytes = reader.get_scan_field_correction_bytes(0).unwrap();
        assert_eq!(bytes.len(), 1_138_799);
    }

    #[test]
    fn get_raw_group_returns_empty_map_for_missing_path() {
        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let result = reader.get_raw_group("OPCUA").unwrap();
        assert!(result.is_empty());
        let result = reader.get_raw_group("does/not/exist").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn reference_fixture_has_no_opcua() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        assert!(config.opcua.is_none());
    }

    #[test]
    fn opcua_fixture_parses_client_pipe_and_triggers() {
        let config = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let opcua = config.opcua.expect("OPCUA group present on this fixture");

        assert!(!opcua.client.server_url.is_empty());
        assert!(opcua.client.bfs_max_depth > 0);
        assert_eq!(opcua.triggers_enabled, Some(true));
        assert!(opcua.triggers.contains_key("Laser Emission Interlock"));

        let trigger = &opcua.triggers["Laser Emission Interlock"];
        assert!(trigger.signal.is_some());
        assert!(trigger.extra.contains_key("Trigger_Label"));
    }

    #[test]
    fn get_raw_group_on_opcua_client() {
        let reader = MachineConfigReader::open(REFERENCE_OPCUA).unwrap();
        let attrs = reader.get_raw_group("OPCUA/Client").unwrap();
        assert!(attrs.contains_key("Server_URL"));
    }

    #[test]
    fn synthetic_fixture_parses() {
        let config = MachineConfigReader::open(SYNTHETIC).unwrap().parse().unwrap();
        assert!(!config.meta.machine_name.is_empty());
        assert_eq!(config.optical_trains.len(), 2);
    }

    #[test]
    fn to_json_default_excludes_binary_fields() {
        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let json = reader.to_json(true, false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert!(parsed.get("meta").is_some());
        assert!(!json.contains("\"correction_data\""));
        assert!(!json.contains("\"raw_bytes\""));
        assert!(!json.contains("\"opcua\""));
    }

    #[test]
    fn to_json_include_binary_adds_correction_and_raw_bytes() {
        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let json = reader.to_json(false, true).unwrap();
        assert!(json.contains("\"correction_data\""));
        assert!(json.contains("\"raw_bytes\""));
    }

    #[test]
    fn matches_python_golden_file() {
        let golden_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_output.json");
        let golden: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(golden_path).unwrap()).unwrap();

        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let ours: serde_json::Value =
            serde_json::from_str(&reader.to_json(true, false).unwrap()).unwrap();

        assert_eq!(ours, golden, "Rust reader output diverges from the Python golden file");
    }
}
