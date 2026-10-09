use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::CapabilityError;
use opcua_config_ext::{
    write_opcua_config, ClientConfig, PipeConfig, TriggerConditionConfig, TriggerConfig,
    TriggerRuleConfig,
};

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
fn write_rejects_duplicate_rule_label() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Same".into(),
        ..Default::default()
    });
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 2,
        rule_label: "Same".into(),
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
fn write_rejects_duplicate_condition_label_within_one_rule() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let cond_a = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Same".into(),
        ..Default::default()
    };
    let cond_b = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Same".into(),
        ..Default::default()
    };

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Unique".into(),
        conditions: vec![cond_a, cond_b],
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
fn write_accepts_duplicate_condition_label_across_multiple_rules() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let cond_a = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Same".into(),
        ..Default::default()
    };
    let cond_b = TriggerConditionConfig {
        condition_id: 2,
        condition_label: "Same".into(),
        ..Default::default()
    };
    let rule_a = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "A".into(),
        conditions: vec![cond_a],
        ..Default::default()
    };
    let rule_b = TriggerRuleConfig {
        rule_id: 2,
        rule_label: "B".into(),
        conditions: vec![cond_b],
        ..Default::default()
    };

    let triggers = TriggerConfig {
        rules: vec![rule_a, rule_b],
        ..Default::default()
    };

    let result = write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    );
    assert!((result.is_ok()));

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules.len(), 2);
    assert_eq!(read_back.rules[0].conditions[0].condition_label, "Same");
    assert_eq!(read_back.rules[1].conditions[0].condition_label, "Same");
}

#[test]
fn write_accepts_duplicate_condition_ids_across_multiple_rules() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let cond_a = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "One".into(),
        ..Default::default()
    };
    let cond_b = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Two".into(),
        ..Default::default()
    };
    let rule_a = TriggerRuleConfig {
        rule_id: 1,
        rule_label: "A".into(),
        conditions: vec![cond_a],
        ..Default::default()
    };
    let rule_b = TriggerRuleConfig {
        rule_id: 2,
        rule_label: "B".into(),
        conditions: vec![cond_b],
        ..Default::default()
    };

    let triggers = TriggerConfig {
        rules: vec![rule_a, rule_b],
        ..Default::default()
    };

    let result = write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    );
    assert!((result.is_ok()));

    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();
    assert_eq!(read_back.rules.len(), 2);
    assert_eq!(read_back.rules[0].conditions[0].condition_id, 1);
    assert_eq!(read_back.rules[1].conditions[0].condition_id, 1);
}

#[test]
fn write_rejects_rule_label_containing_slash() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Foo/Bar".into(),
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
fn write_rejects_condition_label_containing_slash() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let cond_a = TriggerConditionConfig {
        condition_id: 1,
        condition_label: "Foo/Bar".into(),
        ..Default::default()
    };

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Unique".into(),
        conditions: vec![cond_a],
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
fn write_rejects_label_containing_nul_byte() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "Bad\0Label".into(),
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
