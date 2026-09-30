use super::*;
use crate::reader::read_opcua_config;
use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;

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
