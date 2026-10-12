//! Extension configuration types
//!
//! `OpcuaConfig` holds the only current extension's types, integration with the
//! MCL's LoadedConfig for it's own modeled types enables streamlined extension support

use serde::{Deserialize, Serialize};

/// Generic attribute value that can be stored in HDF5
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Float(f64),
    Int(i64),
    FloatArray(Vec<f64>),
    Bool(bool),
}

/// OPCUA configuration from /OPCUA/Client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientConfig {
    // CLIENT_GROUP
    /// Sets the user identity token type required for authentication.
    /// OPC-UA's UserTokenType governs how identity is proven, not authorization/RBAC.
    /// Range: `[Anonymous, UsernamePassword, Certificate]`
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
    /// Range: `[Aconity, Eos]`
    #[serde(rename = "Machine_Profile")]
    pub machine_profile: String,
    /// Time between batch dispatch from server to client [ms]
    #[serde(rename = "Publish_Interval")]
    pub publish_interval: u32,
    /// Queue filtering policy to prevent buffer overflow
    /// Range: { DropOldest, KeepOldest, other behaviors in-development }
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
    /// The target node for a subset of a server's address space. Three resolution modes,
    /// checked in order: (1) contains `=` -> NodeId/ExpandedNodeId string (`i=85`,
    /// `ns=2;s=500300`, `nsu=<uri>;s=<id>`), no browsing needed; (2) non-empty, no `=` ->
    /// fleet container BrowseName, browsed for under ObjectsFolder; (3) empty -> auto-discovery
    /// (tries the default `"MachineFleet"` BrowseName first, else single-candidate discovery).
    /// For an EOS profile this should be mode 1 (NodeId format) — falling through to mode 2's
    /// Aconity-shaped fleet lookup for EOS is a real, previously-recurring misconfiguration.
    #[serde(rename = "Root_Node")]
    pub root_node: String,
    /// How often existing subscriptions are polled [ms]. Real default is profile-dependent:
    /// `50` for Aconity, `-1` ("server-negotiated"), `0` ("as fast as possible") for EOS/static-address-
    /// space profiles — i32 specifically to allow that `-1` sentinel.
    #[serde(rename = "Sampling_Interval")]
    pub sampling_interval: i32,
    /// Secure level of data in transport.
    /// Range: `None` / `Sign` / `SignAndEncrypt`
    #[serde(rename = "Security_Mode")]
    pub security_mode: String,
    /// Level of encryption for client-server connection.
    /// Range: `Basic256Sha256` / `ECC_nistP256` / `ECC_nistP384` / `ECC_brainpoolP256r1` /
    /// `ECC_brainpoolP384r1` — empty lets the SDK auto-select the strongest policy the server
    /// supports, which is the real client-proj default (not a pinned value).
    #[serde(rename = "Security_Policy")]
    pub security_policy: String,
    /// Specifies target server.
    /// Format: `opc.tcp://IP_ADDR:PORT/CONFIG_TEMPLATE_ID/SERVER_INSTANCE_NAME` - the path is
    /// server-implementation-defined, not OPC-UA-spec-mandated. Confirmed against client-proj's
    /// real values: the first segment mirrors its `configSectionName` (e.g.
    /// `TM_OPCUA_DevTemplate_V0.1`), the second is the server instance name (e.g.
    /// `TelemetryServer`).
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
            auth_mode: "Anonymous".into(),
            bfs_max_depth: 16,
            keep_alive_count: 240,
            lifetime_count: 2400,
            machine_profile: String::new(),
            publish_interval: 250,
            queue_policy: "DiscardOldest".into(),
            queue_size_data_change: 10,
            queue_size_events: 10,
            reconnect_interval: 1000,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipeConfig {
    // PIPE_GROUP
    /// Maximum size of by pipe buffer [Bytes]
    #[serde(rename = "Buffer_Size")]
    pub buffer_size: u32,
    /// Enable/Disable pushing machine config Client Group entries to client.
    /// Bool-as-u32: `1` pushes config to client, `0` disables the push (CLI overrides only).
    #[serde(rename = "Configure_Client")]
    pub configure_client: u32,
    /// Limits processing rate of read loop [Hz]. `0` and `-1` currently both mean no additional
    /// cap beyond whatever the client's own pacing already bounds things to; `>0` rate-limits
    /// to N Hz. Pipe-side throttle, conceptually parallel to the OPC UA session's own
    /// sampling-interval negotiation, not literally tied to it yet - see the NOTE below.
    /// NOTE: a separate setting/mechanism could be introduced to act as a pause, leaving
    /// capture or the client in a connected but not consuming state, distinct from this
    /// field's rate semantics.
    /// NOTE: eventually `-1` could mean inherit the client's negotiated rate for real, once
    /// client-proj's hot/warm/cold config-reload path captures server-negotiated settings -
    /// e.g. the subscription's actual, revised publishing interval, which can differ from the
    /// requested `publish_interval` - and reports them back to capture as an initial
    /// config-confirmation message over the pipe. That would let all 3 pipeline components
    /// [server, client, capture] converge on the same effective value where it matters,
    /// instead of capture only ever seeing the client's static, requested config.
    #[serde(rename = "Inbound_Rate_Limit")]
    pub inbound_rate_limit: i32,
    /// Maximum size of single message [Bytes]
    #[serde(rename = "Max_Inbound_Message_Size")]
    pub max_inbound_message_size: u32,
    /// Minimum client process integrity level accepted by pipe.
    /// Range: [Untrusted, Low, Medium, High, System, ProtectedProcess]
    /// (the real winnt.h-documented range - nothing above `ProtectedProcess` is a named level).
    /// An unrecognized value fails toward `ProtectedProcess`, the strictest option - matching
    /// `user_access_level`'s fail-safe direction below.
    #[serde(rename = "Min_Integrity_Level")]
    pub min_integrity_level: String,
    /// Enable/Disable for OPCUA support
    #[serde(rename = "Pipe_Enabled")]
    pub pipe_enabled: u32,
    /// Name of pipe to match client. Windows-name format (`\\.\pipe\...`) - Linux reformats
    /// this automatically into its own UDS socket path convention.
    #[serde(rename = "Pipe_Name")]
    pub pipe_name: String,
    /// DACL restriction based on user level.
    /// Range: [CurrentUserOnly, CurrentUserAndAdmins, AnyLocalUser, Unrestricted].
    /// Stored/documented PascalCase; every consumer normalizes to lowercase before matching.
    /// An unrecognized value fails safe downstream (capture's `PipeSecurity::from` falls back
    /// to `CurrentUserOnly`, the strictest option).
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
            min_integrity_level: "ProtectedProcess".into(),
            pipe_enabled: 0,
            pipe_name: r"\\.\pipe\opc_ua_client_pipe".into(),
            user_access_level: "CurrentUserOnly".into(),
        }
    }
}

/// OPCUA configuration from /OPCUA/Trigger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TriggerConfig {
    // TRIGGER_GROUP
    /// Global trigger gate.
    /// Bool-as-u32: `1` enables trigger-based capture, `0` is manual-capture-only.
    #[serde(rename = "Triggers_Enabled")]
    pub triggers_enabled: u32,
    /// Readiness override/capture requirement fallback
    #[serde(rename = "Trigger_Stop_Ceiling_Layers")]
    pub trigger_stop_ceiling_layers: u32,
    /// Rules for trigger actions
    #[serde(default)]
    pub rules: Vec<TriggerRuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
/// Trigger configuration from /OPCUA/Trigger/{TriggerLabel}
pub struct TriggerRuleConfig {
    // RULE_SUBGROUP
    /// Determines what automation capture performs based if rule evaluates.
    /// Range: `CaptureStart` / `CaptureStop`
    #[serde(rename = "Action")]
    pub action: String,
    /// Determines whether hybrid evaluation is active.
    /// Range: `And` / `Or`
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
    /// Enable/Disable for per-rule support.
    /// Bool-as-u32: `1` enables this rule for triggering, `0` disables it.
    #[serde(rename = "Rule_Enabled")]
    pub rule_enabled: u32,
    /// Stable identifier for the rule
    #[serde(rename = "Rule_Id")]
    pub rule_id: u32,
    /// User-facing label for a rule - intended to capture the machine behavior the rule applies to
    #[serde(rename = "Rule_Label")]
    pub rule_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
/// Trigger configuration from /OPCUA/Triggers/{TriggerLabel}
pub struct TriggerConditionConfig {
    // CONDITION_SUBGROUP
    /// Affects how strictly to evaluate rule based on matching signal name.
    /// Range: `Exact` / `Contains` / `ExactCaseSensitive` / `ContainsCaseSensitive`
    #[serde(rename = "Case_Sensitivity", default)]
    pub case_sensitivity: String,
    /// Numeric specific equivalent to case_sensitivity.
    /// Range: `Equal` / `GreaterThan` / `LessThan` / `GreaterThanOrEqual` / `LessThanOrEqual`
    #[serde(rename = "Comparison_Operator", default)]
    pub comparison_operator: String,
    /// Component for which this rule applies
    #[serde(rename = "Component", default)]
    pub component: String,
    /// Enable/Disable for per-condition support.
    /// Bool-as-u32: `1` enables this condition for its rule, `0` disables it.
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
    /// Determines trigger evaluation logic.
    /// Range: `String` / `Numeric`
    #[serde(rename = "Evaluation_Mode", default)]
    pub evaluation_mode: String,
    /// Event type for which this rule applies
    #[serde(rename = "Event", default)]
    pub event: String,
    /// Signal for which this rule applies
    #[serde(rename = "Signal")]
    pub signal: String,
    /// Subsystem [component group] for which this rule applies
    #[serde(rename = "Subsystem", default)]
    pub subsystem: String,
    /// Value that requests rule evaluation
    #[serde(rename = "Value")]
    pub value: String,
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
