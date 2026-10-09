use opcua_config_ext::{
    read_opcua_config, write_opcua_config, ClientConfig, PipeConfig, TriggerConditionConfig,
    TriggerConfig, TriggerRuleConfig,
};
use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;

#[test]
fn write_prunes_a_rule_removed_since_the_last_write() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut first = TriggerConfig::default();
    first.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "A".into(),
        ..Default::default()
    });
    first.rules.push(TriggerRuleConfig {
        rule_id: 2,
        rule_label: "B".into(),
        ..Default::default()
    });
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &first,
    )
    .unwrap();

    let mut second = TriggerConfig::default();
    second.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "A".into(),
        ..Default::default()
    });
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &second,
    )
    .unwrap();

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules.len(), 1);
    assert_eq!(read_back.rules[0].rule_label, "A");
}

#[test]
fn write_prunes_a_condition_removed_since_the_last_write() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut first = TriggerConfig::default();
    first.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        conditions: vec![
            TriggerConditionConfig {
                condition_id: 1,
                condition_label: "KeepMe".into(),
                ..Default::default()
            },
            TriggerConditionConfig {
                condition_id: 2,
                condition_label: "RemoveMe".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &first,
    )
    .unwrap();

    let mut second = TriggerConfig::default();
    second.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        conditions: vec![TriggerConditionConfig {
            condition_id: 1,
            condition_label: "KeepMe".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &second,
    )
    .unwrap();

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules[0].conditions.len(), 1);
    assert_eq!(read_back.rules[0].conditions[0].condition_label, "KeepMe");
}

#[test]
fn write_prunes_all_rules_down_to_empty() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut first = TriggerConfig::default();
    first.rules.push(TriggerRuleConfig { rule_id: 1, rule_label: "A".into(), ..Default::default() });
    first.rules.push(TriggerRuleConfig { rule_id: 2, rule_label: "B".into(), ..Default::default() });
    write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &first).unwrap();

    let empty = TriggerConfig::default();
    write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &empty).unwrap();

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert!(read_back.rules.is_empty());
}

#[test]
fn write_is_idempotent_across_repeated_identical_writes() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1, rule_label: "Rule".into(),
        conditions: vec![TriggerConditionConfig { condition_id: 1, condition_label: "C".into(), ..Default::default() }],
        ..Default::default()
    });

    write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &triggers).unwrap();
    write_opcua_config(&adapter, &PipeConfig::default(), &ClientConfig::default(), &triggers).unwrap();

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules.len(), 1);
    assert_eq!(read_back.rules[0].conditions.len(), 1);
}
