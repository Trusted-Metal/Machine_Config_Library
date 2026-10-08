# Trigger config design: what the schema itself enables

This is a companion to [README.md](./README.md#field-reference)'s field
reference, one level up: not which fields are required vs. defaulted, but what
*capabilities* the shape of the schema buys, and specifically what it buys
over the stale core `OpcuaTrigger`/`OpcuaConfig` (`rust/src/models.rs`). How
capture actually *evaluates* this data at runtime — matching, edge detection,
cooldown, action dispatch — is out of scope here; that's
[clearbox-tauri/docs/OPC UA/TRIGGER_SYSTEM.md](../../../clearbox-tauri/docs/OPC%20UA/TRIGGER_SYSTEM.md).
This doc is about what the *config* is capable of expressing in the first
place, independent of how any one consumer chooses to interpret it.

## Layout at a glance

```
PipeConfig                        ClientConfig                    TriggerConfig
───────────                       ────────────                    ─────────────
buffer_size              u32      auth_mode              String   triggers_enabled              u32
configure_client         u32      bfs_max_depth           u32     trigger_stop_ceiling_layers   u32
inbound_rate_limit       i32      keep_alive_count        u32     rules                   Vec<TriggerRuleConfig>  
max_inbound_message_size u32      lifetime_count          u32    
min_integrity_level      String   machine_profile        String                      
pipe_enabled             u32      publish_interval        u32     ┌─ TriggerRuleConfig ────────────────────┐
pipe_name                String   queue_policy           String   │ action                       String    │
user_access_level        String   queue_size_data_change  u32     │ combinator                   String    │
                                  queue_size_events       u32     │ conditions Vec<TriggerConditionConfig> |
                                  reconnect_interval      u32     │ cooldown_period              u32       │
                                  root_node              String   │ description                  String    │
                                  sampling_interval       i32     │ max_fires_per_job            u32       │
                                  security_mode          String   │ priority                     u32       │
                                  security_policy        String   │ rule_enabled                 u32       │
                                  server_url             String   │ rule_id                      u32       │
                                  session_timeout         u32     │ rule_label                  String     │
                                  sync_loop_interval_initial u32  └────────────────────────────────────────┘
                                  sync_loop_interval_settled u32  ┌─ TriggerConditionConfig ────────────┐
                                                                  │ case_sensitivity            String  │
                                                                  │ comparison_operator          String │
                                                                  │ component                   String  │
                                                                  │ condition_enabled            u32    │
                                                                  │ condition_id                 u32    │
                                                                  │ condition_label             String  │
                                                                  │ epsilon                      f64    │
                                                                  │ evaluation_mode             String  │
                                                                  │ event                       String  │
                                                                  │ signal                      String  │
                                                                  │ subsystem                   String  │
                                                                  │ value                       String  │
                                                                  └─────────────────────────────────────┘
```

Nesting: `TriggerConfig.rules: Vec<TriggerRuleConfig>`, and each rule's
`conditions: Vec<TriggerConditionConfig>` — a rule owns zero or more
conditions; a condition belongs to exactly one rule.

## What the stale `OpcuaTrigger` actually looks like, for contrast

```rust
pub struct OpcuaTrigger {
    pub id: Option<String>,
    pub signal: Option<String>,
    pub subsystem: Option<String>,
    pub rule_enabled: Option<bool>,
    pub start_value: Option<String>,
    pub stop_value: Option<String>,
    pub case_sensitivity: Option<String>,
    pub component: Option<String>,
    pub cooldown_period: Option<i64>,
    pub event: Option<String>,
    pub max_fires_per_job: Option<i64>,
    pub trigger_label: Option<String>,
    pub extra: ExtraAttrs,
}
```
Flat, single-signal, and every field including `id`/`rule_enabled` is
`Option` — there's no required identity at all beyond the `IndexMap` key
(the HDF5 group name), and no field is ever enforced present. Every
comparison below is against this actual shape, not an assumption about it.

## Feature 1: multiple conditions per rule, combined with `combinator`

`OpcuaTrigger` has exactly one `signal` — a stale "trigger" can only ever react
to a single data point. There is no way to express "fire when A *or* B is
true" within one trigger at all; that would have to be built as two separate
trigger entries with no shared action semantics, or encoded as ad hoc logic
entirely outside the schema.

The new schema's `TriggerRuleConfig.conditions` is a list, combined by
`combinator` ("And"/"Or"). The real fixture this session validated against
demonstrates exactly why that matters: the `Job Stop-Finish` rule has
`Combinator: "Or"` over two conditions — `RunState == "finished"` and
`RunState == "stopped"` — one stop action, fired by either signal transition.
Under the stale shape this would need to be two independent triggers with no
way to express that they're the same logical stop condition.

## Feature 2: heterogeneous conditions within one rule

Because each condition independently carries its own `signal`/`event`/
`component`/`subsystem`, a single rule's conditions aren't required to share a
data source. The real `NumericalTesting` rule (action `CaptureStop`) has three
conditions: two watching `RunState` (string comparison) and one watching
`sensor_temperature` from the oxygen sensor subsystem (numeric comparison,
`GreaterThanOrEqual`, threshold `200.0`) — combined with `Or`. One action, one
rule, driven by a genuine mix of machine-state and sensor-threshold signals.
`OpcuaTrigger`'s single `signal`/`subsystem`/`component` triple makes this
structurally impossible — a stale trigger is pinned to one signal, period.

## Feature 3: action-per-rule, not start/stop-value-per-trigger

`OpcuaTrigger` models start and stop as two sibling fields on the *same*
entry: `start_value`/`stop_value`. One trigger, one signal, two watched
values. The new schema instead gives every rule exactly one `action`
(`CaptureStart`/`CaptureStop`), so starting and stopping are two independently
identified, independently enabled, independently throttled rules — never two
values bolted onto one. The real fixture's `Job Start` (action `CaptureStart`)
and `Job Stop-Finish` (action `CaptureStop`) are separate rules, each with its
own `rule_id`/`rule_enabled`/`cooldown_period`, despite both watching the same
underlying `RunState` signal. Disabling the stop behavior without touching the
start behavior (or vice versa) is a direct consequence of this separation,
not something that needs special-casing.

## Feature 4: `evaluation_mode`-switched comparison semantics

`OpcuaTrigger` has only `case_sensitivity` — there is no numeric comparison
concept anywhere in the stale shape. It can express string equality and
nothing else; `sensor_temperature >= 200.0` has no representation.

`TriggerConditionConfig` carries both a string-comparison path
(`case_sensitivity`: Exact/Contains/etc.) and a numeric-comparison path
(`comparison_operator`: GreaterThanOrEqual/LessThan/Equal/etc., with `epsilon`
for float-tolerance), and `evaluation_mode` ("String"/"Numeric") selects which
sibling fields are semantically live for that condition. The type doesn't
enforce the split — both field sets exist on every condition, both have
defaults — the *evaluation* logic does (capture's `atomic_condition_from_config`
branches on `evaluation_mode` before even looking at the other side's fields).
What the schema buys is letting one condition list freely mix string-keyed
machine-state checks and numeric sensor-threshold checks side by side, exactly
as `NumericalTesting`'s three conditions do.

## Feature 5: per-rule throttling plus cross-rule conflict resolution

`cooldown_period` and `max_fires_per_job` exist on both shapes. `priority`
does not exist on `OpcuaTrigger` at all. The new schema's `priority` field
exists specifically for the case two rules could both match the same moment —
its doc comment states the intent directly: "conflict resolution for rules
with the same conditions but differing actions." A stale trigger has no way to
express "if both of these could fire, this one wins."

## Feature 6: two-level, fail-loud enablement

`OpcuaTrigger.rule_enabled` is `Option<bool>` — a trigger can exist with no
enablement state asserted either way. The new schema makes `rule_enabled`
(and `condition_enabled`) required `u32` fields, and pairs them with a
separate, required, genuinely top-level `triggers_enabled` on `TriggerConfig`
itself (distinct from any one rule's own flag) — a global kill-switch
independent of per-rule state. An operator can disable one misbehaving rule
without touching the rest of the trigger subsystem, or disable the whole
subsystem without editing every rule individually. Making both flags required
rather than optional was a deliberate fail-loud call: a config that's silent
about whether a rule is enabled is read as an error here, not as "assume
disabled" or "assume enabled."

## Feature 7: stable numeric identity, decoupled from the display label

`OpcuaTrigger.id`/`trigger_label` are both `Option<String>`, and neither is
actually load-bearing — the `IndexMap<String, OpcuaTrigger>` key (the HDF5
group name) is the only thing that's actually unique, and nothing requires
`id` or `trigger_label` to agree with it or even be populated. The new schema
makes `rule_id`/`condition_id` required `u32` fields, enforced unique (rule
IDs across the whole file, condition IDs within their own rule), independent
of `rule_label`/`condition_label` (also required, and *themselves* enforced
unique, since they double as literal HDF5 group names). A rule can be renamed
— its label changed — without its identity changing, because the numeric ID
is the thing duplicate-detection and any future rename-safe reference would
key on, not the display string. Today's capture-side runtime happens to key
some of its own in-memory state (cooldown maps, edge detection) by label
rather than ID — that's a capture-side implementation choice documented in
`TRIGGER_SYSTEM.md`, not a limitation of what the schema itself makes
available.

## Feature 8: transport, session, and rule config are structurally separate

`Pipe` (IPC transport: buffer sizes, rate limits, named-pipe identity),
`Client` (OPC UA session/subscription tuning: security mode, sampling
interval, queue policy), and `Trigger` (business-logic rules) are three
independent HDF5 groups, not one monolithic blob. Changing a sampling
interval never touches rule data; adding a rule never touches transport
config. `OpcuaConfig` keeps the same three-way split (`client`/`pipe`/
`triggers` fields) — this one isn't a new-schema-only advantage, just a
property both share that's worth stating plainly since it's part of why the
layout above reads the way it does.
