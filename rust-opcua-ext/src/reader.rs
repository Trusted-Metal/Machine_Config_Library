use crate::raw_group::RawGroupAccess;
use crate::types::{
    ClientConfig, PipeConfig, TriggerConditionConfig, TriggerConfig, TriggerRuleConfig,
};
use crate::writer::{validate_bool_u32, validate_enum, validate_enum_ci, validate_label};
use machine_config::capabilities::v1_0::hdf5::{peek_file_version, Hdf5AdapterV1_0};
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;
use machine_config::CapabilityError;
use std::path::Path;

fn io_err<T, E: std::fmt::Display>(r: Result<T, E>) -> Result<T, CapabilityError> {
    r.map_err(|e| CapabilityError::IoError(e.to_string()))
}

pub fn read_opcua_config(
    path: &Path,
) -> Result<(PipeConfig, ClientConfig, TriggerConfig), CapabilityError> {
    let version = peek_file_version(path).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    match version.as_str() {
        "1.1" => read_via(&io_err(Hdf5AdapterV1_1::open(path))?),
        _ => read_via(&io_err(Hdf5AdapterV1_0::open(path))?),
    }
}

// Generic over anything exposing get_raw_group/list_raw_group_children —
// worth promoting to a real shared trait once both adapters have the methods,
// rather than duplicating this dispatch by hand each time a third version shows up.
fn read_via(
    adapter: &impl RawGroupAccess,
) -> Result<(PipeConfig, ClientConfig, TriggerConfig), CapabilityError> {
    let client: ClientConfig =
        parse_attrs(&adapter.get_raw_group(&format!("{}/Client", crate::OPCUA_ROOT))?)?;
    validate_enum(
        "Auth_Mode",
        &client.auth_mode,
        &["Anonymous", "UsernamePassword", "Certificate"],
    )?;
    validate_enum_ci("Machine_Profile", &client.machine_profile, &["Aconity", "Eos"])?;
    validate_enum(
        "Queue_Policy",
        &client.queue_policy,
        &["DiscardOldest", "KeepOldest"],
    )?;
    validate_enum(
        "Security_Mode",
        &client.security_mode,
        &["None", "Sign", "SignAndEncrypt"],
    )?;
    validate_enum(
        "Security_Policy",
        &client.security_policy,
        &[
            "Basic256Sha256",
            "ECC_nistP256",
            "ECC_nistP384",
            "ECC_brainpoolP256r1",
            "ECC_brainpoolP384r1",
        ],
    )?;

    let pipe: PipeConfig =
        parse_attrs(&adapter.get_raw_group(&format!("{}/Pipe", crate::OPCUA_ROOT))?)?;
    validate_bool_u32("Configure_Client", pipe.configure_client)?;
    // Case-insensitive pending verification — see the doc comment on this field for why.
    validate_enum_ci(
        "Min_Integrity_Level",
        &pipe.min_integrity_level,
        &[
            "Untrusted",
            "Low",
            "Medium",
            "High",
            "System",
            "ProtectedProcess",
        ],
    )?;
    validate_bool_u32("Pipe_Enabled", pipe.pipe_enabled)?;
    validate_enum(
        "User_Access_Level",
        &pipe.user_access_level,
        &[
            "CurrentUserOnly",
            "CurrentUserAndAdmins",
            "AnyLocalUser",
            "Unrestricted",
        ],
    )?;

    let mut rules: Vec<TriggerRuleConfig> = Vec::new();

    for rule_name in adapter.list_raw_group_children(&format!("{}/Trigger", crate::OPCUA_ROOT))? {
        let rule_path = format!("{}/Trigger/{rule_name}", crate::OPCUA_ROOT);
        let mut rule: TriggerRuleConfig = parse_attrs(&adapter.get_raw_group(&rule_path)?)?;
        if rule.rule_id == 0 {
            return Err(CapabilityError::validation_error(format!(
                "rule '{rule_name}' has no rule_id"
            )));
        }
        if rule.rule_label.trim().is_empty() {
            rule.rule_label = format!("Rule_{}", rule.rule_id);
        }
        validate_enum("Action", &rule.action, &["CaptureStart", "CaptureStop"])?;
        validate_enum("Combinator", &rule.combinator, &["And", "Or"])?;
        validate_bool_u32("Rule_Enabled", rule.rule_enabled)?;
        validate_label(&rule.rule_label, "rule_label")?;

        for cond_name in adapter.list_raw_group_children(&rule_path)? {
            let cond_attrs = adapter.get_raw_group(&format!("{rule_path}/{cond_name}"))?;
            let mut condition: TriggerConditionConfig = parse_attrs(&cond_attrs)?;
            if condition.condition_id == 0 {
                return Err(CapabilityError::validation_error(format!(
                    "condition '{cond_name}' in rule '{rule_name}' has no condition_id"
                )));
            }
            if condition.condition_label.trim().is_empty() {
                condition.condition_label = format!("Condition_{}", condition.condition_id);
            }
            validate_enum(
                "Case_Sensitivity",
                &condition.case_sensitivity,
                &[
                    "Exact",
                    "ExactCaseSensitive",
                    "Contains",
                    "ContainsCaseSensitive",
                ],
            )?;
            validate_enum(
                "Comparison_Operator",
                &condition.comparison_operator,
                &[
                    "Equal",
                    "GreaterThan",
                    "LessThan",
                    "GreaterThanOrEqual",
                    "LessThanOrEqual",
                ],
            )?;
            validate_bool_u32("Condition_Enabled", condition.condition_enabled)?;
            validate_label(&condition.condition_label, "condition_label")?;
            validate_enum(
                "Evaluation_Mode",
                &condition.evaluation_mode,
                &["String", "Numeric"],
            )?;
            rule.conditions.push(condition);
        }
        rule.conditions.sort_by_key(|c| c.condition_id);
        for pair in rule.conditions.windows(2) {
            if pair[0].condition_id == pair[1].condition_id {
                return Err(CapabilityError::validation_error(format!(
                    "duplicate condition_id {} in rule '{}'",
                    pair[0].condition_id, rule.rule_label
                )));
            }
        }
        rules.push(rule);
    }

    rules.sort_by_key(|r| r.rule_id);
    for pair in rules.windows(2) {
        if pair[0].rule_id == pair[1].rule_id {
            return Err(CapabilityError::validation_error(format!(
                "duplicate rule_id on disk: {}",
                pair[0].rule_id
            )));
        }
    }

    let mut triggers: TriggerConfig =
        parse_attrs(&adapter.get_raw_group(&format!("{}/Trigger", crate::OPCUA_ROOT))?)?;
    validate_bool_u32("Triggers_Enabled", triggers.triggers_enabled)?;
    triggers.rules = rules;
    Ok((pipe, client, triggers))
}

// Deserializes an ExtraAttrs (IndexMap<String, serde_json::Value>) into any
// #[derive(Deserialize)] config struct via a JSON round-trip — cheap, and
// keeps this file from hand-writing a field-by-field reader like MCL's own
// reader.rs does for its typed core fields.
fn parse_attrs<T: serde::de::DeserializeOwned>(
    attrs: &machine_config::ExtraAttrs,
) -> Result<T, CapabilityError> {
    let value = serde_json::to_value(attrs).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    serde_json::from_value(value).map_err(|e| CapabilityError::IoError(e.to_string()))
}
