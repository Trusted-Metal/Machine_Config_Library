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

#[test]
fn round_trip_preserves_every_pipe_client_and_trigger_field() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let pipe = PipeConfig {
        buffer_size: 32768,
        configure_client: 1,
        inbound_rate_limit: -1,
        max_inbound_message_size: 2048,
        min_integrity_level: "high".into(),
        pipe_enabled: 1,
        pipe_name: r"\\.\pipe\test_pipe".into(),
        user_access_level: "CurrentUserOnly".into(),
    };

    let client = ClientConfig {
        auth_mode: "UsernamePassword".into(),
        bfs_max_depth: 8,
        keep_alive_count: 100,
        lifetime_count: 1200,
        machine_profile: "EOS".into(),
        publish_interval: 100,
        queue_policy: "DropYoungest".into(),
        queue_size_data_change: 100,
        queue_size_events: 1000,
        reconnect_interval: 100,
        root_node: "Fleet".into(),
        sampling_interval: 500,
        security_mode: "Sign".into(),
        security_policy: "ECC_brainpoolP384r1".into(),
        server_url: "opc.tcp://ip_address:port/".into(),
        session_timeout: 100,
        sync_loop_interval_initial: 100,
        sync_loop_interval_settled: 100,
    };

    let cond_a = TriggerConditionConfig {
        case_sensitivity: "Exact".into(),
        comparison_operator: "Equal".into(),
        component: "comp1".into(),
        condition_enabled: 1,
        condition_id: 1,
        condition_label: "label_a".into(),
        epsilon: 1.0,
        evaluation_mode: "Numeric".into(),
        event: "event1".into(),
        signal: "signal1".into(),
        subsystem: "subsystem1".into(),
        value: "2".into(),
    };
    let cond_b = TriggerConditionConfig {
        case_sensitivity: "Contains".into(),
        comparison_operator: "LessThan".into(),
        component: "comp2".into(),
        condition_enabled: 1,
        condition_id: 2,
        condition_label: "label_b".into(),
        epsilon: 2.0,
        evaluation_mode: "String".into(),
        event: "event2".into(),
        signal: "signal2".into(),
        subsystem: "subsystem2".into(),
        value: "value2".into(),
    };
    let rule_a = TriggerRuleConfig {
        action: "CaptureStart".into(),
        combinator: "And".into(),
        conditions: vec![cond_a.clone(), cond_b.clone()],
        cooldown_period: 10,
        description: "description1".into(),
        max_fires_per_job: 10,
        priority: 1,
        rule_enabled: 1,
        rule_id: 1,
        rule_label: "rule_a".into(),
    };
    let rule_b = TriggerRuleConfig {
        action: "CaptureStop".into(),
        combinator: "Or".into(),
        conditions: vec![cond_a, cond_b],
        cooldown_period: 10,
        description: "description2".into(),
        max_fires_per_job: 10,
        priority: 1,
        rule_enabled: 1,
        rule_id: 2,
        rule_label: "rule_b".into(),
    };
    let trigger = TriggerConfig {
        triggers_enabled: 1,
        trigger_stop_ceiling_layers: 10,
        rules: vec![rule_a, rule_b],
    };

    write_opcua_config(&adapter, &pipe, &client, &trigger).unwrap();
    let (read_pipe, read_client, read_trigger) = read_opcua_config(tmp.path()).unwrap();

    assert_eq!(read_pipe, pipe);
    assert_eq!(read_client, client);
    assert_eq!(read_trigger, trigger);
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
fn multi_rule_multi_condition_round_trip_is_independent_per_rule() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let triggers = TriggerConfig {
        rules: vec![
            TriggerRuleConfig {
                rule_id: 3,
                rule_label: "Third".into(),
                conditions: vec![
                    TriggerConditionConfig {
                        condition_id: 2,
                        condition_label: "C3b".into(),
                        ..Default::default()
                    },
                    TriggerConditionConfig {
                        condition_id: 1,
                        condition_label: "C3a".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            TriggerRuleConfig {
                rule_id: 1,
                rule_label: "First".into(),
                conditions: vec![TriggerConditionConfig {
                    condition_id: 1,
                    condition_label: "C1a".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            TriggerRuleConfig {
                rule_id: 2,
                rule_label: "Second".into(),
                conditions: vec![
                    TriggerConditionConfig {
                        condition_id: 3,
                        condition_label: "C2c".into(),
                        ..Default::default()
                    },
                    TriggerConditionConfig {
                        condition_id: 1,
                        condition_label: "C2a".into(),
                        ..Default::default()
                    },
                    TriggerConditionConfig {
                        condition_id: 2,
                        condition_label: "C2b".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &triggers,
    )
    .unwrap();
    let (_, _, read_back) = read_opcua_config(tmp.path()).unwrap();

    assert_eq!(
        read_back
            .rules
            .iter()
            .map(|r| r.rule_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(read_back.rules[0].conditions.len(), 1);
    assert_eq!(read_back.rules[2].conditions.len(), 2);

    let rule2 = &read_back.rules[1]; // rule_id 2, written with 3 conditions
    assert_eq!(
        rule2
            .conditions
            .iter()
            .map(|c| c.condition_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(rule2.conditions.iter().all(|c| c.condition_label != "C1a")); // no leakage from rule 1
}

#[test]
fn rule_with_zero_conditions_round_trips_cleanly() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let mut triggers = TriggerConfig::default();
    triggers.rules.push(TriggerRuleConfig {
        rule_id: 1,
        rule_label: "AlwaysFire".into(),
        action: "CaptureStart".into(),
        rule_enabled: 1,
        conditions: Vec::new(),
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

    assert_eq!(read_back.rules.len(), 1);
    assert!(read_back.rules[0].conditions.is_empty());
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

#[test]
fn flaglike_u32_and_string_agree_on_enabled_semantics() {
    assert!(is_flag_enabled(&1u32));
    assert!(!is_flag_enabled(&0u32));
    assert!("true".is_enabled());
    assert!(!"false".is_enabled());
    assert!("TRUE".is_enabled()); // case-insensitivity, explicitly
}

#[test]
fn attribute_value_untagged_enum_variant_fidelity() {
    // Documents real behavior, not the "obviously correct" assumption:
    // Float is tried before Int, so a JSON integer always round-trips as Float.
    let round_tripped: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::Int(5)).unwrap()).unwrap();
    assert_eq!(round_tripped, AttributeValue::Float(5.0)); // NOT Int(5)

    let s: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::String("123".into())).unwrap())
            .unwrap();
    assert_eq!(s, AttributeValue::String("123".into())); // unambiguous, survives correctly

    let b: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::Bool(true)).unwrap()).unwrap();
    assert_eq!(b, AttributeValue::Bool(true));
}

#[test]
fn read_opcua_config_falls_back_to_v1_0_on_unrecognized_file_version() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    make_fixture(tmp.path(), "9.9"); // nonsense — not "1.0", not "1.1"
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap(); // v1.0 shape, matching the fallback

    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();
    assert!(read_opcua_config(tmp.path()).is_ok());
}
