//! Runtime trigger-matching engine — the *evaluated* shape, distinct from
//! `types.rs`'s config/on-disk shape. Ported from clearbox-tauri's
//! trigger/mod.rs (feat/hybrid-trigger), condition-evaluation logic only —
//! capture-default-resolution (EffectiveCaptureDefaults/resolve_*) stays in
//! clearbox-tauri, it's app-specific, not schema.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseSensitivity {
    Exact,
    ExactCaseSensitive,
    Contains,
    ContainsCaseSensitive,
}

impl CaseSensitivity {
    fn values_match(&self, actual: &str, expected: &str) -> bool {
        let (a, e) = (actual.trim(), expected.trim());
        match self {
            Self::Exact => a.eq_ignore_ascii_case(e),
            Self::ExactCaseSensitive => a == e,
            Self::Contains => a.to_ascii_lowercase().contains(&e.to_ascii_lowercase()),
            Self::ContainsCaseSensitive => a.contains(e),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMode {
    Numeric,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SignalKey {
    pub event: Option<String>,
    pub subsystem: Option<String>,
    pub component: Option<String>,
    pub signal: String,
}

impl SignalKey {
    pub fn from_message(message: &serde_json::Value) -> Option<Self> {
        let signal = message.get("Signal").and_then(|v| v.as_str())?.to_string();
        Some(Self {
            event: message
                .get("Event")
                .and_then(|v| v.as_str())
                .map(String::from),
            subsystem: message
                .get("Subsystem")
                .and_then(|v| v.as_str())
                .map(String::from),
            component: message
                .get("Component")
                .and_then(|v| v.as_str())
                .map(String::from),
            signal,
        })
    }
}

pub const EPSILON: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub struct AtomicCondition {
    pub case_sensitivity: CaseSensitivity,
    pub comparison_op: ComparisonOp,
    pub component: Option<String>,
    pub epsilon: f64,
    pub evaluation_mode: EvaluationMode,
    pub event: Option<String>,
    pub signal: String,
    pub subsystem: Option<String>,
    pub value: String,
}

impl AtomicCondition {
    pub fn key(&self) -> SignalKey { 
        SignalKey {
            event: self.event.clone(),
            subsystem: self.subsystem.clone(),
            component: self.component.clone(),
            signal: self.signal.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Atomic(AtomicCondition),
    Compound {
        combinator: Combinator,
        of: Vec<AtomicCondition>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerActionKind {
    CaptureStart,
    CaptureStop,
}
// NOTE: Room for AnnotateEvent/FlushToDisk/NotifyOperator/RunWebhook later — dispatch
// mechanism only, not new capability until a consumer actually needs one.

#[derive(Debug, Clone)]
pub struct TriggerRule {
    pub action: TriggerActionKind,
    pub condition: Condition,
    pub cooldown_period: u32,
    pub enabled: bool,
    pub id: u32,
    pub label: String,
    pub max_fires_per_job: u32,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValue {
    pub raw: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl CachedValue {
    fn as_string(&self) -> String {
        match &self.raw {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match &self.raw {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.trim().parse().ok(),
            serde_json::Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
}

pub fn evaluate_condition(condition: &Condition, cache: &HashMap<SignalKey, CachedValue>) -> bool {
    match condition {
        Condition::Atomic(c) => evaluate_atomic(c, cache),
        Condition::Compound { combinator, of } => match combinator {
            Combinator::And => of.iter().all(|c| evaluate_atomic(c, cache)),
            Combinator::Or => of.iter().any(|c| evaluate_atomic(c, cache)),
        },
    }
}

fn evaluate_atomic(condition: &AtomicCondition, cache: &HashMap<SignalKey, CachedValue>) -> bool {
    if condition.event.is_some() && condition.subsystem.is_some() && condition.component.is_some() {
        return evaluate_value(condition, cache.get(&condition.key()));
    }
    cache
        .iter()
        .filter(|(k, _)| k.signal == condition.signal)
        .filter(|(k, _)| {
            condition
                .event
                .as_deref()
                .map_or(true, |e| k.event.as_deref() == Some(e))
        })
        .filter(|(k, _)| {
            condition
                .subsystem
                .as_deref()
                .map_or(true, |s| k.subsystem.as_deref() == Some(s))
        })
        .filter(|(k, _)| {
            condition
                .component
                .as_deref()
                .map_or(true, |c| k.component.as_deref() == Some(c))
        })
        .any(|(_, cached)| evaluate_value(condition, Some(cached)))
}

fn evaluate_value(condition: &AtomicCondition, cached: Option<&CachedValue>) -> bool {
    let Some(cached) = cached else { return false };
    match condition.evaluation_mode {
        EvaluationMode::String => condition
            .case_sensitivity
            .values_match(&cached.as_string(), &condition.value),
        EvaluationMode::Numeric => {
            let (Some(actual), Some(expected)) =
                (cached.as_f64(), condition.value.trim().parse::<f64>().ok())
            else {
                tracing::warn!(
                    signal = %condition.signal,
                    raw = %cached.raw,
                    "Numeric condition on a non-numeric signal value — treating as unmatched"
                );
                return false;
            };
            let eps = if condition.epsilon > 0.0 {
                condition.epsilon
            } else {
                EPSILON
            };
            match condition.comparison_op {
                ComparisonOp::Equal => (actual - expected).abs() < eps,
                ComparisonOp::NotEqual => (actual - expected).abs() >= eps,
                ComparisonOp::LessThan => actual < expected,
                ComparisonOp::LessThanOrEqual => actual <= expected,
                ComparisonOp::GreaterThan => actual > expected,
                ComparisonOp::GreaterThanOrEqual => actual >= expected,
            }
        }
    }
}
