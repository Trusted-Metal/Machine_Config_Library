# Opcua Configuration Extension

This MCL extension prototype owns reading and writing the OPC UA Pipe/Client/Trigger
configuration schema stored under `Extensions/TM_OPCUA` in a machine-config HDF5 file. It does
**not** own trigger *evaluation* — that system lives in clearbox-tauri's own `trigger`
`opcua::trigger_services` modules (see
[clearbox-tauri/docs/OPC UA/TRIGGER_ENGINE.md](../../../clearbox-tauri/docs/OPC%20UA/TRIGGER_ENGINE.md)).
This crate's job ends at handing back typed, validated config structs — what a consumer does
with them is out of scope by design.

See also:

- [TRIGGER_CONFIG_DESIGN.md](./TRIGGER_CONFIG_DESIGN.md) — what the trigger schema's shape itself
  enables functionally, contrasted feature-by-feature with the legacy `OpcuaTrigger` shape.
- [ARCHITECTURE_RATIONALE.md](./ARCHITECTURE_RATIONALE.md) — why this exists as a separate
  extension crate instead of living in MCL core, *evaluation of transitioning to core vs
  maintaining as a separately/extendining for other extensions is in review*.
- [MAINTENANCE_OPTIONS.md](./MAINTENANCE_OPTIONS.md) — the three paths this crate could take from
  here (stay as-is / split into its own repo / fold into core), and the specific conditions that
  would justify each one.

## HDF5 layout

```text
Extensions/TM_OPCUA/
  Opcua_Schema_Version      (attr on this group itself)
  Pipe/                     (flat attrs — PipeConfig)
  Client/                   (flat attrs — ClientConfig)
  Trigger/                  (flat attrs — TriggerConfig's own two fields, plus...)
    {Rule_Label}/            one child group per rule, named by its OWN label
      {Condition_Label}/      direct child of its rule — no intermediate "Conditions" group
```

An item worth internalizing up front:

- **The root is singular `Trigger`, not `Triggers`.** Out of step with Client/Pipe groups — MCL
  core's own writer wrote `Triggers` while its reader (independently patched earlier) expected
  `Trigger`, and the mismatch wasn't caught until a full test run. `OPCUA_ROOT` (`lib.rs`) and
  every path built from it uses singular `Trigger` consistently; `Triggers_Enabled` as an
  *attribute name* inside that group is unrelated and correctly stays plural, denoting a
  group/vector of rules.

## Field reference

**Strict** (required — reading a file missing one of these is a hard error, not a default): this
was a deliberate call, not an oversight — for reasoning see
[ARCHITECTURE_RATIONALE.md](./ARCHITECTURE_RATIONALE.md#fail-loud-field-strictness)

| Struct | Strict fields |
| --- | --- |
| `PipeConfig` | all 8 fields (no `#[serde(default)]` anywhere on this struct) |
| `ClientConfig` | all 18 fields (same) |
| `TriggerConfig` | both `triggers_enabled` and `trigger_stop_ceiling_layers` |
| `TriggerRuleConfig` | `action`, `rule_enabled`, `rule_id`, `rule_label` |
| `TriggerConditionConfig` | `condition_enabled`, `condition_id`, `condition_label`, `signal`, `value` |

Everything else on `TriggerRuleConfig`/`TriggerConditionConfig` (`combinator`,
`cooldown_period`, `description`, `max_fires_per_job`, `priority`, `case_sensitivity`,
`comparison_operator`, `component`, `epsilon`, `evaluation_mode`, `event`, `subsystem`) is
`#[serde(default)]` — chosen field-by-field against `trigger_services.rs`'s actual downstream
handling, which already has a graceful, logged fallback for each of them.

## Label rules

Labels become literal HDF5 group path segments, which creates two real constraints the writer
enforces (and the reader partially re-checks, since a hand-authored file can't be trusted to have
gone through this writer at all):

- **Uniqueness**: `rule_label` must be unique across all rules in one write; `condition_label`
  only needs to be unique *within its own rule* — the same label on two different rules'
  conditions is fine, since each rule is its own HDF5 parent group. A collision isn't just an
  error for its own sake: two rules sharing a label would silently merge into one HDF5 group,
  with the second write's attrs clobbering the first's.
- **No `/` or NUL byte**: `/` would be read as a path separator, silently turning one labeled
  group into two nested ones instead of erroring; NUL isn't a legal HDF5 group-name character
  at all.

## Full-replace writes (orphan pruning)

`write_opcua_config` is a **complete replace**, not a merge: any rule or condition group that
exists on disk from a previous write but is absent from the current `TriggerConfig` gets
deleted, not left behind. Deleting a rule's group recursively removes its conditions too
(see HDF5 link semantics), so rule/condition-level cleanup separation is sufficient.

## Version dispatch

`read_opcua_config` reads `File_Version` and dispatches `"1.1"` to `Hdf5AdapterV1_1`; everything
else — including `"1.0"` and any unrecognized value — falls back to `Hdf5AdapterV1_0`, matching
MCL core's own fallback convention.

## Producers and consumers

- clearbox-tauri (capture) **permanently** maintains its own independent, hand-rolled reader
  (`machine_config/reader.rs`) rather than ever calling into this crate's `read_opcua_config` —
  this is a deliberate, standing architectural decision, not a migration gap to close later.
- This crate's reader/writer exist for (1) a planned separate authoring/producer utility, and
  (2) third-party adopters of the machine-config format who want portable read/write utilities
  outside clearbox-tauri's own codebase.

## Field value ranges

| Field | Evaluated Type | Default | Range |
| `auth_mode` | `enum` | `Certificate` | `Anonymous, UsernamePassword, Certificate` |
