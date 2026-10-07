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

    let mut triggers = TriggerConfig {
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

    let mut triggers = TriggerConfig {
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
        rule_label: "rule_a".into()
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
        rule_label: "rule_b".into()
    };
    let trigger = TriggerConfig {
        triggers_enabled: 1,
        trigger_stop_ceiling_layers: 10,
        rules: vec![rule_a, rule_b],
    };

    write_opcua_config(&adapter, &pipe, &client, &trigger).unwrap();
    let (read_pipe, read_client, read_trigger) = read_opcua_config(tmp.path()).unwrap();

    assert_eq!(read_pipe.buffer_size, pipe.buffer_size);
    assert_eq!(read_pipe.configure_client, pipe.configure_client);
    assert_eq!(read_pipe.inbound_rate_limit, pipe.inbound_rate_limit);
    assert_eq!(read_pipe.max_inbound_message_size, pipe.max_inbound_message_size);
    assert_eq!(read_pipe.min_integrity_level, pipe.min_integrity_level);
    assert_eq!(read_pipe.pipe_enabled, pipe.pipe_enabled);
    assert_eq!(read_pipe.pipe_name, pipe.pipe_name);
    assert_eq!(read_pipe.user_access_level, pipe.user_access_level);

    assert_eq!(read_client.auth_mode, client.auth_mode);
    assert_eq!(read_client.bfs_max_depth, client.bfs_max_depth);
    assert_eq!(read_client.keep_alive_count, client.keep_alive_count);
    assert_eq!(read_client.lifetime_count, client.lifetime_count);
    assert_eq!(read_client.machine_profile, client.machine_profile);
    assert_eq!(read_client.publish_interval, client.publish_interval);
    assert_eq!(read_client.queue_policy, client.queue_policy);
    assert_eq!(read_client.queue_size_data_change, client.queue_size_data_change);
    assert_eq!(read_client.queue_size_events, client.queue_size_events);
    assert_eq!(read_client.reconnect_interval, client.reconnect_interval);
    assert_eq!(read_client.root_node, client.root_node);
    assert_eq!(read_client.sampling_interval, client.sampling_interval);
    assert_eq!(read_client.security_mode, client.security_mode);
    assert_eq!(read_client.security_policy, client.security_policy);
    assert_eq!(read_client.server_url, client.server_url);
    assert_eq!(read_client.session_timeout, client.session_timeout);
    assert_eq!(read_client.sync_loop_interval_initial, client.sync_loop_interval_initial);
    assert_eq!(read_client.sync_loop_interval_settled, client.sync_loop_interval_settled);

    assert_eq!(read_trigger.triggers_enabled, trigger.triggers_enabled);
    assert_eq!(read_trigger.trigger_stop_ceiling_layers, trigger.trigger_stop_ceiling_layers);
    assert_eq!(read_trigger.rules.len(), 2);
    assert_eq!(read_trigger.rules[0].action, rule_a.action);
    assert_eq!(read_trigger.rules[0].combinator, rule_a.combinator);
    assert_eq!(read_trigger.rules[0].cooldown_period, rule_a.cooldown_period);
    assert_eq!(read_trigger.rules[0].description, rule_a.description);
    assert_eq!(read_trigger.rules[0].max_fires_per_job, rule_a.max_fires_per_job);
    assert_eq!(read_trigger.rules[0].priority, rule_a.priority);
    assert_eq!(read_trigger.rules[0].rule_enabled, rule_a.rule_enabled);
    assert_eq!(read_trigger.rules[0].rule_id, rule_a.rule_id);
    assert_eq!(read_trigger.rules[0].rule_label, rule_a.rule_label);
    assert_eq!(read_trigger.rules[1].action, rule_b.action);
    assert_eq!(read_trigger.rules[1].combinator, rule_b.combinator);
    assert_eq!(read_trigger.rules[1].cooldown_period, rule_b.cooldown_period);
    assert_eq!(read_trigger.rules[1].description, rule_b.description);
    assert_eq!(read_trigger.rules[1].max_fires_per_job, rule_b.max_fires_per_job);
    assert_eq!(read_trigger.rules[1].priority, rule_b.priority);
    assert_eq!(read_trigger.rules[1].rule_enabled, rule_b.rule_enabled);
    assert_eq!(read_trigger.rules[1].rule_id, rule_b.rule_id);
    assert_eq!(read_trigger.rules[1].rule_label, rule_b.rule_label);
    
    assert_eq!(read_trigger.rules[0].conditions.len(), 2);
    assert_eq!(read_trigger.rules[0].conditions[0].case_sensitivity, cond_a.case_sensitivity);
    assert_eq!(read_trigger.rules[0].conditions[0].comparison_operator, cond_a.comparison_operator);
    assert_eq!(read_trigger.rules[0].conditions[0].component, cond_a.component);
    assert_eq!(read_trigger.rules[0].conditions[0].condition_enabled, cond_a.condition_enabled);
    assert_eq!(read_trigger.rules[0].conditions[0].condition_id, cond_a.condition_id);
    assert_eq!(read_trigger.rules[0].conditions[0].condition_label, cond_a.condition_label);
    assert_eq!(read_trigger.rules[0].conditions[0].epsilon, cond_a.epsilon);
    assert_eq!(read_trigger.rules[0].conditions[0].evaluation_mode, cond_a.evaluation_mode);
    assert_eq!(read_trigger.rules[0].conditions[0].event, cond_a.event);
    assert_eq!(read_trigger.rules[0].conditions[0].signal, cond_a.signal);
    assert_eq!(read_trigger.rules[0].conditions[0].subsystem, cond_a.subsystem);
    assert_eq!(read_trigger.rules[0].conditions[0].value, cond_a.value);
    assert_eq!(read_trigger.rules[0].conditions[1].case_sensitivity, cond_b.case_sensitivity);
    assert_eq!(read_trigger.rules[0].conditions[1].comparison_operator, cond_b.comparison_operator);
    assert_eq!(read_trigger.rules[0].conditions[1].component, cond_b.component);
    assert_eq!(read_trigger.rules[0].conditions[1].condition_enabled, cond_b.condition_enabled);
    assert_eq!(read_trigger.rules[0].conditions[1].condition_id, cond_b.condition_id);
    assert_eq!(read_trigger.rules[0].conditions[1].condition_label, cond_b.condition_label);
    assert_eq!(read_trigger.rules[0].conditions[1].epsilon, cond_b.epsilon);
    assert_eq!(read_trigger.rules[0].conditions[1].evaluation_mode, cond_b.evaluation_mode);
    assert_eq!(read_trigger.rules[0].conditions[1].event, cond_b.event);
    assert_eq!(read_trigger.rules[0].conditions[1].signal, cond_b.signal);
    assert_eq!(read_trigger.rules[0].conditions[1].subsystem, cond_b.subsystem);
    assert_eq!(read_trigger.rules[0].conditions[1].value, cond_b.value);
}
