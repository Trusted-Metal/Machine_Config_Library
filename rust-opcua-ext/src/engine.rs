//! Runtime trigger-matching engine — the *evaluated* shape, distinct from
//! `types.rs`'s config/on-disk shape. Ported from clearbox-tauri's
//! trigger/mod.rs (feat/hybrid-trigger), condition-evaluation logic only —
//! capture-default-resolution (EffectiveCaptureDefaults/resolve_*) stays in
//! clearbox-tauri, it's app-specific, not schema.

use std::collections::HashMap;

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
    pub fn key(&self) -> SignalKey { /* unchanged */
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
// Room for AnnotateEvent/FlushToDisk/NotifyOperator/RunWebhook later — dispatch
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
// as_string()/as_f64() — unchanged from source.

pub fn evaluate_condition(condition: &Condition, cache: &HashMap<SignalKey, CachedValue>) -> bool {
    /* unchanged — Atomic dispatches to evaluate_atomic; Compound And/Or over of.iter() */
}
// evaluate_atomic/evaluate_value — unchanged, ported verbatim.
