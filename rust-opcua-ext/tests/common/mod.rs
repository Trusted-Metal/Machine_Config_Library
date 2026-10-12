// Not itself a test file — pulled in via `mod common;`. Cargo only treats
// files directly under tests/ as independent test binaries; a tests/common/
// subdirectory is the standard way to share code between them.
// read_opcua_config/write_opcua_config/the config types are used inside round_trip_test!'s
// body, but that macro expands at its *call site* (roundtrip.rs), which already imports them
// itself — nothing in this file uses them directly, so importing them here too is unused.
use machine_config::builder::MockConfigBuilder;

pub fn make_fixture(path: &std::path::Path, file_version: &str) {
    MockConfigBuilder::new(1).save(path).unwrap();
    if file_version != "1.0" {
        let f = hdf5::File::open_rw(path).unwrap();
        let root = f.group("/").unwrap();
        root.delete_attr("File_Version").unwrap();
        let vlu: hdf5::types::VarLenUnicode = file_version.parse().unwrap();
        root.new_attr::<hdf5::types::VarLenUnicode>()
            .create("File_Version")
            .unwrap()
            .write_scalar(&vlu)
            .unwrap();
    }
}

#[macro_export]
macro_rules! round_trip_test {
    ($name:ident, $version:literal, $adapter:ty) => {
        #[test]
        fn $name() {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            $crate::common::make_fixture(tmp.path(), $version);
            let adapter = <$adapter>::open(tmp.path()).unwrap();

            let mut triggers = TriggerConfig::default();
            triggers.rules.push(TriggerRuleConfig {
                rule_label: "Laser_Interlock".into(),
                rule_id: 1,
                conditions: vec![TriggerConditionConfig {
                    condition_label: "Yellow_Light".into(),
                    condition_id: 1,
                    ..Default::default()
                }],
                ..Default::default()
            });

            write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &triggers)
                .unwrap();
            let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
            assert_eq!(read_back.rules[0].rule_label, "Laser_Interlock");
            assert_eq!(read_back.rules[0].conditions[0].condition_label, "Yellow_Light");
        }
    };
}
