use opcua_config_ext::{is_flag_enabled, AttributeValue, FlagLike};

#[test]
fn flaglike_u32_and_string_agree_on_enabled_semantics() {
    assert!(is_flag_enabled(&1u32));
    assert!(!is_flag_enabled(&0u32));
    assert!("true".is_enabled());
    assert!(!"false".is_enabled());
    assert!("TRUE".is_enabled());
}

#[test]
fn attribute_value_untagged_enum_variant_fidelity() {
    let round_tripped: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::Int(5)).unwrap()).unwrap();
    assert_eq!(round_tripped, AttributeValue::Float(5.0));

    let s: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::String("123".into())).unwrap())
            .unwrap();
    assert_eq!(s, AttributeValue::String("123".into()));

    let b: AttributeValue =
        serde_json::from_value(serde_json::to_value(AttributeValue::Bool(true)).unwrap()).unwrap();
    assert_eq!(b, AttributeValue::Bool(true));
}
