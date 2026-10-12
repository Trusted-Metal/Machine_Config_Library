mod common;

use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;
use opcua_config_ext::{
    read_opcua_config, write_opcua_config, ClientConfig, PipeConfig, TriggerConditionConfig,
    TriggerConfig, TriggerRuleConfig,
};

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
fn empty_trigger_config_round_trips_cleanly() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    MockConfigBuilder::new(1).save(tmp.path()).unwrap();
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    let triggers = TriggerConfig::default();
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
        queue_policy: "KeepOldest".into(),
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

round_trip_test!(round_trips_v1_0, "1.0", Hdf5AdapterV1_0);
round_trip_test!(round_trips_v1_1, "1.1", Hdf5AdapterV1_1);

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

    let rule2 = &read_back.rules[1];
    assert_eq!(
        rule2
            .conditions
            .iter()
            .map(|c| c.condition_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(rule2.conditions.iter().all(|c| c.condition_label != "C1a"));
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
fn read_opcua_config_falls_back_to_v1_0_on_unrecognized_file_version() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    common::make_fixture(tmp.path(), "9.9");
    let adapter = Hdf5AdapterV1_0::open(tmp.path()).unwrap();

    write_opcua_config(
        &adapter,
        &PipeConfig::default(),
        &ClientConfig::default(),
        &TriggerConfig::default(),
    )
    .unwrap();
    assert!(read_opcua_config(tmp.path()).is_ok());
}
