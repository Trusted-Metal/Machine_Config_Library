//! Extension configuration types
//!
//! `OpcuaConfig` holds the only current extension's types, integration with the
//! MCL's LoadedConfig for it's own modeled types enables streamlined extension support

use serde::{Deserialize, Serialize};

/// Generic attribute value that can be stored in HDF5
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Float(f64),
    Int(i64),
    FloatArray(Vec<f64>),
    Bool(bool),
}

/// OPCUA configuration from /OPCUA/Client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    // CLIENT_GROUP
    /// Sets user identity token requirements for RBAC
    #[serde(rename = "Auth_Mode")]
    pub auth_mode: String,
    /// Sets search depth for address space discovery and subscription
    #[serde(rename = "BFS_Max_Depth")]
    pub bfs_max_depth: u32,
    /// Maximum number of consecutive publish cycles without any new data before server sends keep-alive confimation
    #[serde(rename = "Keep_Alive_Count")]
    pub keep_alive_count: u32,
    /// Maximum number of consecutive publish cycles without successfule request before subscription termination
    #[serde(rename = "Lifetime_Count")]
    pub lifetime_count: u32,
    /// OEM/Make specifier which selects browse strategy and other subsets of client processes/properties
    #[serde(rename = "Machine_Profile")]
    pub machine_profile: String,
    /// Time between batch dispatch from server to client [ms]
    #[serde(rename = "Publish_Interval")]
    pub publish_interval: u32,
    /// Queue filtering policy to prevent buffer overflow
    #[serde(rename = "Queue_Policy")]
    pub queue_policy: String,
    /// Maximum number of data change items
    #[serde(rename = "Queue_Size_Data_Change")]
    pub queue_size_data_change: u32,
    /// Maximum number of event type items
    #[serde(rename = "Queue_Size_Events")]
    pub queue_size_events: u32,
    /// How often client attempts server reconnect [ms]
    #[serde(rename = "Reconnect_Interval")]
    pub reconnect_interval: u32,
    /// The target node for a subset of a server's address space
    #[serde(rename = "Root_Node")]
    pub root_node: String,
    /// How often existing subscriptions are polled [ms]
    #[serde(rename = "Sampling_Interval")]
    pub sampling_interval: i32,
    /// Secure level of data in transport
    #[serde(rename = "Security_Mode")]
    pub security_mode: String,
    /// Level of encryption for client-server connection
    #[serde(rename = "Security_Policy")]
    pub security_policy: String,
    /// Specifies target server
    #[serde(rename = "Server_URL")]
    pub server_url: String,
    /// How long until client connection timesout [ms]
    #[serde(rename = "Session_Timeout")]
    pub session_timeout: u32,
    /// How often the client checks for new subscriptions [ms]
    #[serde(rename = "Sync_Loop_Interval_Initial")]
    pub sync_loop_interval_initial: u32,
    /// How often client checks for new subscriptions after settling [ms]
    #[serde(rename = "Sync_Loop_Interval_Settled")]
    pub sync_loop_interval_settled: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            // CLIENT_GROUP
            auth_mode: "Certificate".into(),
            bfs_max_depth: 16,
            keep_alive_count: 240,
            lifetime_count: 2400,
            machine_profile: String::new(),
            publish_interval: 250,
            queue_policy: "DropOldest".into(),
            queue_size_data_change: 10,
            queue_size_events: 7200,
            reconnect_interval: 10000,
            root_node: String::new(),
            sampling_interval: 250,
            security_mode: "SignAndEncrypt".into(),
            security_policy: "Basic256Sha256".into(),
            server_url: String::new(),
            session_timeout: 60000,
            sync_loop_interval_initial: 1000,
            sync_loop_interval_settled: 30000,
        }
    }
}

/// OPCUA configuration from /OPCUA/Pipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeConfig {
    // PIPE_GROUP
    /// Maximum size of by pipe buffer [Bytes]
    #[serde(rename = "Buffer_Size")]
    pub buffer_size: u32,
    /// Enable/Disable pushing machine config Client Group entries to client
    #[serde(rename = "Configure_Client")]
    pub configure_client: u32,
    /// Limits processing rate of read loop [Hz]
    #[serde(rename = "Inbound_Rate_Limit")]
    pub inbound_rate_limit: i32,
    /// Maximum size of single message [Bytes]
    #[serde(rename = "Max_Inbound_Message_Size")]
    pub max_inbound_message_size: u32,
    /// Minimum client process integrity level accepted by pipe
    #[serde(rename = "Min_Integrity_Level")]
    pub min_integrity_level: String,
    /// Enable/Disable for OPCUA support
    #[serde(rename = "Pipe_Enabled")]
    pub pipe_enabled: u32,
    /// Name of pipe to match client
    #[serde(rename = "Pipe_Name")]
    pub pipe_name: String,
    /// DACL restriction based on user level
    #[serde(rename = "User_Access_Level")]
    pub user_access_level: String,
}

impl Default for PipeConfig {
    fn default() -> Self {
        Self {
            // PIPE_GROUP
            buffer_size: 65536,
            configure_client: 0,
            inbound_rate_limit: 100,
            max_inbound_message_size: 4096,
            min_integrity_level: "medium".into(),
            pipe_enabled: 0,
            pipe_name: r"\\.\pipe\opc_ua_client_pipe".into(),
            user_access_level: "AnyLocalUser".into(),
        }
    }
}

/// OPCUA configuration from /OPCUA/Trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    // TRIGGER_GROUP
    /// Global trigger gate
    #[serde(rename = "Triggers_Enabled")]
    pub triggers_enabled: u32,
    /// Readiness override/capture requirement fallback
    #[serde(rename = "Trigger_Stop_Ceiling_Layers")]
    pub trigger_stop_ceiling_layers: u32,
    /// Rules for trigger actions
    #[serde(default)]
    pub rules: Vec<TriggerRuleConfig>,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            // TRIGGER_GROUP
            triggers_enabled: 0,
            trigger_stop_ceiling_layers: 0,
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Trigger configuration from /OPCUA/Trigger/{TriggerLabel}
pub struct TriggerRuleConfig {
    // RULE_SUBGROUP
    /// Determines what automation capture performs based if rule evaluates
    #[serde(rename = "Action")]
    pub action: String,
    /// Determines whether hybrid evaluation is active
    #[serde(rename = "Combinator", default)]
    pub combinator: String,
    /// Conditions for rule evaluation
    #[serde(default)]
    pub conditions: Vec<TriggerConditionConfig>,
    /// Minimum time between trigger firing consecutively [ms]
    #[serde(rename = "Cooldown_Period", default)]
    pub cooldown_period: u32,
    /// Operator-facing notes for internal justification for rule existence
    #[serde(rename = "Description", default)]
    pub description: String,
    /// Maximum number of firings between a start/stop evaluation
    #[serde(rename = "Max_Fires_Per_Job", default)]
    pub max_fires_per_job: u32,
    /// Conflict resolution for rules with the same conditions but differing actions
    #[serde(rename = "Priority", default)]
    pub priority: u32,
    /// Enable/Disable for per-rule support
    #[serde(rename = "Rule_Enabled")]
    pub rule_enabled: u32,
    /// Stable identifier for the rule
    #[serde(rename = "Rule_Id")]
    pub rule_id: u32,
    /// User-facing label for a rule - intended to capture the machine behavior the rule applies to
    #[serde(rename = "Rule_Label")]
    pub rule_label: String,
}

impl Default for TriggerRuleConfig {
    fn default() -> Self {
        Self {
            action: String::new(),
            combinator: String::new(),
            conditions: Vec::new(),
            cooldown_period: 0,
            description: String::new(),
            max_fires_per_job: 0,
            priority: 0,
            rule_enabled: 0,
            rule_id: 0,
            rule_label: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Trigger configuration from /OPCUA/Triggers/{TriggerLabel}
pub struct TriggerConditionConfig {
    // CONDITION_SUBGROUP
    /// Affects how strictly to evaluate rule based on matching signal name
    #[serde(rename = "Case_Sensitivity", default)]
    pub case_sensitivity: String,
    /// Numeric specific equivalent to case_sensitivity
    #[serde(rename = "Comparison_Operator", default)]
    pub comparison_operator: String,
    /// Component for which this rule applies
    #[serde(rename = "Component", default)]
    pub component: String,
    /// Enable/Disable for per-condition support
    #[serde(rename = "Condition_Enabled")]
    pub condition_enabled: u32,
    /// Stable identifier for this condition within its rule — never assume
    /// `conditions` is in the same order the HDF5 group was written in.
    #[serde(rename = "Condition_Id", default)]
    pub condition_id: u32,
    /// User-facing label for a condition - intended to capture the signal behavior the condition evaluates on
    #[serde(rename = "Condition_Label")]
    pub condition_label: String,
    /// Determines sensitivity to float value delta
    #[serde(rename = "Epsilon", default)]
    pub epsilon: f64,
    /// Determines trigger evaluation logic
    #[serde(rename = "Evaluation_Mode", default)]
    pub evaluation_mode: String,
    /// Event type for which this rule applies
    #[serde(rename = "Event", default)]
    pub event: String,
    /// Signal for which this rule applies
    #[serde(rename = "Signal", default)]
    pub signal: String,
    /// Subsystem [component group] for which this rule applies
    #[serde(rename = "Subsystem", default)]
    pub subsystem: String,
    /// Value that requests rule evaluation
    #[serde(rename = "Value")]
    pub value: String,
}

impl Default for TriggerConditionConfig {
    fn default() -> Self {
        Self {
            case_sensitivity: String::new(),
            comparison_operator: String::new(),
            component: String::new(),
            condition_enabled: 0,
            condition_id: 0,
            condition_label: String::new(),
            epsilon: 0.0,
            evaluation_mode: String::new(),
            event: String::new(),
            signal: String::new(),
            subsystem: String::new(),
            value: String::new(),
        }
    }
}

/// Simplifies repeated checks for (opcua-config) flag guards
pub trait FlagLike {
    fn is_enabled(&self) -> bool;
}

impl FlagLike for str {
    fn is_enabled(&self) -> bool {
        self.eq_ignore_ascii_case("true")
    }
}

impl FlagLike for String {
    fn is_enabled(&self) -> bool {
        self.as_str().is_enabled()
    }
}

impl FlagLike for u32 {
    fn is_enabled(&self) -> bool {
        *self != 0
    }
}

pub fn is_flag_enabled<T: FlagLike + ?Sized>(v: &T) -> bool {
    v.is_enabled()
}
