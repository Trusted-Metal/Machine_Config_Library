use super::*;
use crate::reader::read_opcua_config;
use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;
use machine_config::CapabilityError;

#[test]
fn rules_are_sorted_by_id_not_write_order() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 2,
        rule_label: "Second".into(),
        ..Default::default()
    });
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "First".into(),
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

    assert_eq!(read_back.rules.len(), 2);
    assert_eq!(read_back.rules[0].rule_id, 1);
    assert_eq!(read_back.rules[1].rule_id, 2);
}

#[test]
fn write_rejects_duplicate_rule_id() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "A".into(),
        ..Default::default()
    });
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "B".into(),
        ..Default::default()
    });

    let result = write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    );
    assert!(matches!(
        result,
        Err(CapabilityError::ValidationError { .. })
    ));
}

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
fn write_rejects_duplicate_condition_id_within_one_rule() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Rule".into(),
        conditions: vec![
            TriggerConditionConfig {
                condition_id: 1,
                condition_label: "A".into(),
                ..Default::default()
            },
            TriggerConditionConfig {
                condition_id: 1,
                condition_label: "B".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    });

    let result = write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    );
    assert!(matches!(
        result,
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
fn empty_trigger_config_round_trips_cleanly() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let triggers = TriggerConfig::default(); // triggers_enabled: "false", zero rules
    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    )
    .unwrap();
    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();

    assert_eq!(read_back.triggers_enabled, 0);
    assert!(read_back.rules.is_empty());
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
            .create("File_Version")
            .unwrap()
            .write_scalar(&vlu)
            .unwrap();
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
    };
}

round_trip_test!(round_trips_v1_0, "1.0", Hdf5AdapterV1_0);
round_trip_test!(round_trips_v1_1, "1.1", Hdf5AdapterV1_1);
