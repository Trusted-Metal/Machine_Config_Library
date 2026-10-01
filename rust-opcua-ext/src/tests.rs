use super::*;
use crate::reader::read_opcua_config;
use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;

#[test]
fn round_trips_pipe_client_triggers() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

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

    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    )
    .unwrap();
    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules[0].rule_label, "Laser_Interlock");
    assert_eq!(
        read_back.rules[0].conditions[0].condition_label,
        "Yellow_Light"
    );
}

fn make_fixture(path: &std::path::Path, file_version: &str) {
    MockConfigBuilder::new(1).save(path).unwrap();
    if file_version != "1.0" {
        let f = hdf5::File::open_rw(path).unwrap();
        let root = f.group("/").unwrap();
        root.delete_attr("File_Version").unwrap();
        let vlu: hdf5::types::VarLenUnicode = file_version.parse().unwrap();
        root.new_attr::<hdf5::types::VarLenUnicode>()
            .create("File_Version").unwrap()
            .write_scalar(&vlu).unwrap();
    }
}

macro_rules! round_trip_test {
    ($name:ident, $version:literal, $adapter:ty) => {
        #[test]
        fn $name() {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            make_fixture(tmp.path(), $version);
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

            write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &triggers).unwrap();
            let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
            assert_eq!(read_back.rules[0].rule_label, "Laser_Interlock");
            assert_eq!(read_back.rules[0].conditions[0].condition_label, "Yellow_Light");
        }
    };
}

round_trip_test!(round_trips_v1_0, "1.0", Hdf5AdapterV1_0);
round_trip_test!(round_trips_v1_1, "1.1", Hdf5AdapterV1_1);

