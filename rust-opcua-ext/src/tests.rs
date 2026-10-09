use super::*;
use crate::reader::read_opcua_config;
use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;
use machine_config::CapabilityError;

#[test]
fn read_rejects_duplicate_rule_id_already_on_disk() {
    // Simulates a file from the separate authoring tool (or hand-edited),
    // bypassing this crate's own writer — the reader is the last line of
    // defense against this, not just the writer's own check.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();

    let rule_a = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule_A".into(),
        ..Default::default()
    };
    let rule_b = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule_B".into(),
        ..Default::default()
    };
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule_A",
            &crate::writer::to_attrs(&rule_a).unwrap(),
        )
        .unwrap();
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule_B",
            &crate::writer::to_attrs(&rule_b).unwrap(),
        )
        .unwrap();

    assert!(matches!(
        read_opcua_config(tmp.path()),
        Err(CapabilityError::ValidationError { .. })
    ));
}

#[test]
fn read_rejects_duplicate_condition_id_already_on_disk() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();

    let rule = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        ..Default::default()
    };
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule",
            &crate::writer::to_attrs(&rule).unwrap(),
        )
        .unwrap();

    let cond_a = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Cond_A".into(),
        ..Default::default()
    };
    let cond_b = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Cond_B".into(),
        ..Default::default()
    };
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule/Cond_A",
            &crate::writer::to_attrs(&cond_a).unwrap(),
        )
        .unwrap();
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule/Cond_B",
            &crate::writer::to_attrs(&cond_b).unwrap(),
        )
        .unwrap();

    assert!(matches!(
        read_opcua_config(tmp.path()),
        Err(CapabilityError::ValidationError { .. })
    ));
}

#[test]
fn missing_label_fallback_to_id_template() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();

    let rule = TriggerRuleConfig {
        rule_id: 1,
        rule_label: String::new(),
        ..Default::default()
    };
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Some_Group_Name",
            &crate::writer::to_attrs(&rule).unwrap(),
        )
        .unwrap();

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules[0].rule_label, "Rule_1");
}

#[test]
fn read_errors_on_missing_required_value_field() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();

    let rule = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        ..Default::default()
    };
    adapter
        .set_raw_group(
            "Extensions/TM_OPCUA/Trigger/Rule",
            &crate::writer::to_attrs(&rule).unwrap(),
        )
        .unwrap();

    let cond = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Cond".into(),
        value: "irrelevant".into(),
        ..Default::default()
    };
    let mut attrs = crate::writer::to_attrs(&cond).unwrap();
    attrs.shift_remove("Value"); // the one condition field with no #[serde(default)]
    adapter
        .set_raw_group("Extensions/TM_OPCUA/Trigger/Rule/Cond", &attrs)
        .unwrap();

    assert!(matches!(
        read_opcua_config(tmp.path()),
        Err(CapabilityError::IoError(_))
    ));
}

#[test]
fn read_errors_on_missing_rule_enabled() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();

    let rule = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        ..Default::default()
    };
    let mut attrs = crate::writer::to_attrs(&rule).unwrap();
    attrs.shift_remove("Rule_Enabled");
    adapter
        .set_raw_group("Extensions/TM_OPCUA/Trigger/Rule", &attrs)
        .unwrap();

    assert!(matches!(
        read_opcua_config(tmp.path()),
        Err(CapabilityError::IoError(_))
    ));
}

#[test]
fn to_attrs_excludes_nested_vec_fields() {
    let rule = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        conditions: vec![TriggerConditionConfig {
            condition_id: 1,
            condition_label: "C".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let attrs = crate::writer::to_attrs(&rule).unwrap();

    assert!(!attrs.contains_key("conditions"));
    assert!(!attrs.contains_key("Conditions"));
    assert!(attrs.contains_key("Rule_Id"));
    assert!(attrs.contains_key("Rule_Label"));
}
