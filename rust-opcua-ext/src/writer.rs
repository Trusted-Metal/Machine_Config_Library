use crate::raw_group::RawGroupAccess;
use crate::types::{ClientConfig, PipeConfig, TriggerConfig};
use machine_config::{CapabilityError, ExtraAttrs};

pub fn write_opcua_config(
    adapter: &impl RawGroupAccess,
    pipe: &PipeConfig,
    client: &ClientConfig,
    triggers: &TriggerConfig,
) -> Result<(), CapabilityError> {
    let mut opcua_attrs = ExtraAttrs::new();
    opcua_attrs.insert("Opcua_Schema_Version".into(), "1.2".into());
    adapter.set_raw_group(crate::OPCUA_ROOT, &opcua_attrs)?;
    adapter.set_raw_group(&format!("{}/Client", crate::OPCUA_ROOT), &to_attrs(client)?)?;
    validate_enum(
        "Auth_Mode",
        &client.auth_mode,
        &["Anonymous", "UsernamePassword", "Certificate"],
    )?;
    validate_enum_ci(
        "Machine_Profile",
        &client.machine_profile,
        &["Aconity", "Eos"],
    )?;
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

    adapter.set_raw_group(&format!("{}/Pipe", crate::OPCUA_ROOT), &to_attrs(pipe)?)?;
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

    let trigger_root = format!("{}/Trigger", crate::OPCUA_ROOT);
    adapter.set_raw_group(&trigger_root, &to_attrs(triggers)?)?;
    validate_bool_u32("Triggers_Enabled", triggers.triggers_enabled)?;

    // Snapshot BEFORE writing anything new — a rule dropped from
    // `triggers.rules` since the last write would otherwise silently survive
    // on disk and reappear on the next read, since list_raw_group_children
    // just reports whatever physically exists, not what the caller intended.
    let existing_rule_names: std::collections::HashSet<String> = adapter
        .list_raw_group_children(&trigger_root)?
        .into_iter()
        .collect();

    let mut seen_rule_ids = std::collections::HashSet::new();
    let mut seen_rule_labels = std::collections::HashSet::new();
    for rule in &triggers.rules {
        if rule.rule_id == 0 {
            return Err(CapabilityError::validation_error(
                "rule_id is required (0 is the unset sentinel)",
            ));
        }
        if rule.rule_label.trim().is_empty() {
            return Err(CapabilityError::validation_error(format!(
                "rule_id {} has an empty rule_label — refusing to write (would collide with the Trigger group itself)",
                rule.rule_id
            )));
        }
        if !seen_rule_ids.insert(rule.rule_id) {
            return Err(CapabilityError::validation_error(format!(
                "duplicate rule_id: {}",
                rule.rule_id
            )));
        }
        if !seen_rule_labels.insert(rule.rule_label.clone()) {
            return Err(CapabilityError::validation_error(format!(
                "duplicate rule_label '{}' — would silently overwrite the earlier \
                 rule's HDF5 group instead of creating a new one",
                rule.rule_label
            )));
        }
        validate_enum("Action", &rule.action, &["CaptureStart", "CaptureStop"])?;
        validate_enum("Combinator", &rule.combinator, &["And", "Or"])?;
        validate_bool_u32("Rule_Enabled", rule.rule_enabled)?;
        validate_label(&rule.rule_label, "rule_label")?;

        let rule_path = format!("{}/Trigger/{}", crate::OPCUA_ROOT, rule.rule_label);
        let existing_condition_names: std::collections::HashSet<String> = adapter
            .list_raw_group_children(&rule_path)?
            .into_iter()
            .collect();
        adapter.set_raw_group(&rule_path, &to_attrs(rule)?)?;

        let mut seen_condition_ids = std::collections::HashSet::new();
        let mut seen_condition_labels = std::collections::HashSet::new();
        for cond in &rule.conditions {
            if cond.condition_id == 0 {
                return Err(CapabilityError::validation_error(format!(
                    "condition_id is required in rule '{}'",
                    rule.rule_label
                )));
            }
            if cond.condition_label.trim().is_empty() {
                return Err(CapabilityError::validation_error(format!(
                    "condition_id {} in rule '{}' has an empty condition_label — refusing to write",
                    cond.condition_id, rule.rule_label
                )));
            }
            if !seen_condition_ids.insert(cond.condition_id) {
                return Err(CapabilityError::validation_error(format!(
                    "duplicate condition_id {} in rule '{}'",
                    cond.condition_id, rule.rule_label
                )));
            }
            if !seen_condition_labels.insert(cond.condition_label.clone()) {
                return Err(CapabilityError::validation_error(format!(
                    "duplicate condition_label '{}' in rule '{}' — would silently \
                     overwrite the earlier condition's HDF5 group",
                    cond.condition_label, rule.rule_label
                )));
            }
            validate_enum(
                "Case_Sensitivity",
                &cond.case_sensitivity,
                &[
                    "Exact",
                    "ExactCaseSensitive",
                    "Contains",
                    "ContainsCaseSensitive",
                ],
            )?;
            validate_enum(
                "Comparison_Operator",
                &cond.comparison_operator,
                &[
                    "Equal",
                    "GreaterThan",
                    "LessThan",
                    "GreaterThanOrEqual",
                    "LessThanOrEqual",
                ],
            )?;
            validate_bool_u32("Condition_Enabled", cond.condition_enabled)?;
            validate_label(&cond.condition_label, "condition_label")?;
            validate_enum(
                "Evaluation_Mode",
                &cond.evaluation_mode,
                &["String", "Numeric"],
            )?;

            let cond_path = format!("{rule_path}/{}", cond.condition_label);
            adapter.set_raw_group(&cond_path, &to_attrs(cond)?)?;
        }

        for stale in existing_condition_names.difference(&seen_condition_labels) {
            adapter.remove_raw_group(&format!("{rule_path}/{stale}"))?;
        }
    }

    for stale in existing_rule_names.difference(&seen_rule_labels) {
        adapter.remove_raw_group(&format!("{trigger_root}/{stale}"))?;
    }

    Ok(())
}

pub(crate) fn to_attrs<T: serde::Serialize>(v: &T) -> Result<ExtraAttrs, CapabilityError> {
    let value = serde_json::to_value(v).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    let serde_json::Value::Object(map) = value else {
        return Err(CapabilityError::IoError("expected a JSON object".into()));
    };
    let mut attrs = ExtraAttrs::new();
    for (k, v) in map {
        if !matches!(
            v,
            serde_json::Value::Array(_) | serde_json::Value::Object(_)
        ) {
            attrs.insert(k, v);
        }
    }
    Ok(attrs)
}

pub(crate) fn validate_label(label: &str, what: &str) -> Result<(), CapabilityError> {
    if label.contains('/') {
        return Err(CapabilityError::validation_error(format!(
            "{what} '{label}' contains '/', which HDF5 reads as a path separator \
             — rename it to avoid splitting into nested groups"
        )));
    }
    if label.contains('\0') {
        return Err(CapabilityError::validation_error(format!(
            "{what} '{label}' contains a NUL byte, which HDF5 group names cannot contain"
        )));
    }
    Ok(())
}

pub(crate) fn validate_enum(field: &str, value: &str, valid: &[&str]) -> Result<(), CapabilityError> {
    if value.is_empty() || valid.contains(&value) {
        return Ok(());
    }
    Err(CapabilityError::validation_error(format!(
        "{field} '{value}' is not one of the recognized values ({}) — set it explicitly before writing",
        valid.join(", ")
    )))
}

/// Case-insensitive variant — only `machine_profile` actually needs this. Its real downstream
/// consumer (`MachineProfile.Resolve` in client-proj) compares via `.ToUpperInvariant()`, so any
/// casing of a recognized value is accepted, and anything else (not just empty) falls back to
/// Aconity rather than erroring. Every other enum field's real consumer does exact-string
/// matching, so this stays a dedicated exception rather than the default behavior.
pub(crate) fn validate_enum_ci(
    field: &str,
    value: &str,
    valid: &[&str],
) -> Result<(), CapabilityError> {
    if value.is_empty() || valid.iter().any(|v| v.eq_ignore_ascii_case(value)) {
        return Ok(());
    }
    Err(CapabilityError::validation_error(format!(
        "{field} '{value}' is not one of the recognized values ({}, case-insensitive) — set it explicitly before writing",
        valid.join(", ")
    )))
}

pub(crate) fn validate_bool_u32(field: &str, value: u32) -> Result<(), CapabilityError> {
    if value > 1 {
        return Err(CapabilityError::validation_error(format!(
            "{field} = {value} is not a valid bool-as-u32 (expected 0 or 1)"
        )));
    }
    Ok(())
}
