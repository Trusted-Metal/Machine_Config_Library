# Schema Reference — machine_config_v1

The canonical schema lives at `schema/machine_config_v1.schema.json`
(JSON Schema draft 2020-12). Every language validates against it; Python uses `jsonschema`,
Node.js uses Ajv, Rust enforces it implicitly via `serde`, and C++ uses pboettch/json-schema-validator.

← [Back to index](../USAGE.md)

---

## Contents

- [Top-level structure](#top-level-structure)
- [meta](#meta)
- [machine](#machine)
- [optical_trains items](#optical_trains-items)
  - [scanner](#scanner)
  - [axis subgroups (x_axis / y_axis / z_axis / focus)](#axis-subgroups)
  - [light_source](#light_source)
  - [collimator](#collimator)
  - [scanner_card](#scanner_card)
  - [optional_components.clearbox](#optional_componentsclearbox)
  - [scan_field_correction_file](#scan_field_correction_file)
- [opcua (non-schema, HDF5 only)](#opcua-non-schema-hdf5-only)
- [Conventions](#conventions)

---

## Top-level structure

```json
{
  "meta":           { ... },          // required
  "machine":        { ... },          // required
  "optical_trains": [ ... ]           // required, 1–6 items
}
```

`optical_trains` must have between 1 and 6 items (schema constraint). All other top-level keys
are disallowed by the schema — OPCUA data lives outside the canonical JSON and is accessed via
the model's `opcua` field or `get_raw_group()`.

---

## meta

All fields are **required**.

| Field | Type | Notes |
|---|---|---|
| `schema_version` | `string` | Always `"v1"` |
| `machine_name` | `string` | Human-readable machine identifier |
| `manufacturer` | `string` | Machine manufacturer |
| `model` | `string` | Machine model |
| `serial_number` | `string` | Machine serial number |
| `file_version` | `string` | HDF5 file format version (e.g. `"1.0"`) |
| `export_date` | `string` (date-time) | ISO 8601 timestamp of when the file was exported |
| `configuration_hash` | `string` | Exactly 64 hex characters — SHA-256 of the configuration |
| `extra` | `object` | Additional vendor-specific attributes not in the schema; values must be `string`, `integer`, or `number` |

---

## machine

All fields are **optional** (`string | null` or `number | null`).

| Field | Type | Notes |
|---|---|---|
| `id` | `string\|null` | Internal machine identifier |
| `machine_name` | `string\|null` | Mirrors `meta.machine_name` |
| `manufacturer` | `string\|null` | |
| `model` | `string\|null` | |
| `serial_number` | `string\|null` | |
| `build_plate_x` | `number\|null` | Build volume X dimension |
| `build_plate_x_unit` | `string\|null` | Always `"mm"` for AconityMIDI fixtures |
| `build_plate_y` | `number\|null` | |
| `build_plate_y_unit` | `string\|null` | |
| `build_plate_z` | `number\|null` | |
| `build_plate_z_unit` | `string\|null` | |
| `build_plate_radius` | `number\|null` | For circular build plates |
| `build_plate_radius_unit` | `string\|null` | |
| `gas_flow_direction` | `string\|null` | e.g. `"Y+"` |
| `recoat_direction` | `string\|null` | e.g. `"X+"` |

---

## optical_trains items

Each item is an **object** with these required fields: `train_id`, `scanner`, `light_source`,
`collimator`, `scanner_card`.

| Field | Type | Notes |
|---|---|---|
| `train_id` | `string` | **Required.** e.g. `"Optical_Train_01"` |
| `beam_waist_major` | `number\|null` | |
| `beam_waist_major_unit` | `string\|null` | |
| `beam_waist_minor` | `number\|null` | |
| `beam_waist_minor_unit` | `string\|null` | |
| `beam_waist_offset_z` | `number\|null` | |
| `beam_waist_offset_z_unit` | `string\|null` | |
| `m2_major` | `number\|null` | M² beam quality factor, major axis |
| `m2_minor` | `number\|null` | |
| `rayleigh_length_major` | `number\|null` | |
| `rayleigh_length_major_unit` | `string\|null` | |
| `rayleigh_length_minor` | `number\|null` | |
| `rayleigh_length_minor_unit` | `string\|null` | |
| `thermal_lensing_passed` | `boolean\|null` | Whether thermal lensing test passed |
| `thermal_lensing_focal_plane_shift` | `number\|null` | |
| `thermal_lensing_focal_plane_shift_unit` | `string\|null` | |
| `thermal_lensing_threshold` | `number\|null` | |
| `thermal_lensing_threshold_unit` | `string\|null` | |
| `scanner_number` | `string\|null` | |

---

### scanner

**Required** on each optical train item.

| Field | Type | Notes |
|---|---|---|
| `manufacturer` | `string` | **Required** |
| `model` | `string` | **Required** |
| `serial_number` | `string` | **Required** |
| `working_distance` | `number\|null` | Distance from scanner to build plane |
| `working_distance_unit` | `string\|null` | Always `"mm"` |
| `scan_field_x` | `number\|null` | Scan field X dimension |
| `scan_field_x_unit` | `string\|null` | |
| `scan_field_y` | `number\|null` | |
| `scan_field_y_unit` | `string\|null` | |
| `scan_field_z` | `number\|null` | |
| `scan_field_z_unit` | `string\|null` | |
| `scan_head_offset_x` | `number\|null` | Scanner centre offset from build-plate centre, X |
| `scan_head_offset_x_unit` | `string\|null` | Always `"mm"` |
| `scan_head_offset_y` | `number\|null` | |
| `scan_head_offset_y_unit` | `string\|null` | |
| `scan_head_offset_z` | `number\|null` | |
| `scan_head_offset_z_unit` | `string\|null` | |
| `scan_head_rotation` | `number\|null` | Rotation angle in degrees; 0° for train 0, 180° for train 1 on AconityMIDI |
| `scan_head_rotation_unit` | `string\|null` | Always `"deg"` |
| `axis_configuration` | `string\|null` | `"2D"`, `"3D"`, or `"3D+Focus"` — drives which axis subgroups are required |
| `x_axis` | object\|null | See [axis subgroups](#axis-subgroups) |
| `y_axis` | object\|null | |
| `z_axis` | object\|null | Present for `"3D"` and `"3D+Focus"` |
| `focus` | object\|null | Present only for `"3D+Focus"` |

**Axis configuration rules** (enforced by `allOf` constraints in the schema):
- `"2D"` → `x_axis` and `y_axis` are required
- `"3D"` → `x_axis`, `y_axis`, and `z_axis` are required
- `"3D+Focus"` → `x_axis`, `y_axis`, `z_axis`, and `focus` are all required

---

### Axis subgroups

Applies to `x_axis`, `y_axis`, `z_axis`, and `focus`. All fields are optional.

| Field | Type |
|---|---|
| `actual_bit_resolution` | `integer\|null` |
| `actual_bit_resolution_unit` | `string\|null` |
| `commanded_bit_resolution` | `integer\|null` |
| `commanded_bit_resolution_unit` | `string\|null` |
| `control_type` | `string\|null` |
| `range_of_motion` | `number\|null` |
| `range_of_motion_unit` | `string\|null` |
| `smoothing_kernel` | `string\|null` |
| `smoothing_parameters` | `number\|null` |
| `tuning_parameters` | `string\|null` |
| `tuning_type` | `string\|null` |

---

### light_source

**Required** on each optical train item.

| Field | Type | Notes |
|---|---|---|
| `manufacturer` | `string` | **Required** |
| `model` | `string` | **Required** |
| `serial_number` | `string` | **Required** |
| `wavelength` | `number\|null` | Laser wavelength |
| `wavelength_unit` | `string\|null` | Always `"nm"` |
| `power_max_nominal` | `number\|null` | |
| `power_max_nominal_unit` | `string\|null` | |
| `power_max_actual` | `number\|null` | |
| `power_max_actual_unit` | `string\|null` | |
| `power_min_actual` | `number\|null` | |
| `power_min_actual_unit` | `string\|null` | |
| `power_min_nominal` | `number\|null` | |
| `power_min_nominal_unit` | `string\|null` | |
| `power_bit_resolution` | `number\|null` | |
| `power_bit_resolution_unit` | `string\|null` | |
| `watts_to_volts_algorithm` | `string\|null` | |
| `watts_to_volts_params` | `string\|null` | |

---

### collimator

**Required** on each optical train item.

| Field | Type | Notes |
|---|---|---|
| `manufacturer` | `string` | **Required** |
| `model` | `string` | **Required** |
| `serial_number` | `string` | **Required** |
| `focal_length` | `number\|null` | |
| `focal_length_unit` | `string\|null` | |

---

### scanner_card

**Required** on each optical train item.

| Field | Type | Notes |
|---|---|---|
| `manufacturer` | `string` | **Required** |
| `model` | `string` | **Required** |
| `serial_number` | `string` | **Required** |
| `communication_protocol` | `string\|null` | e.g. `"SP-ICE-3"` |
| `sample_period` | `number\|null` | |
| `sample_period_unit` | `string\|null` | e.g. `"μs"` |

---

### optional_components.clearbox

Present when the machine has a ClearBox unit. `additionalProperties: false` — no extra keys
are permitted.

| Field | Type | Notes |
|---|---|---|
| `ip_address` | `string` | **Required.** IP address of the ClearBox unit |
| `serial_number` | `string\|null` | |
| `data_port` | `integer\|null` | Default 5001 |
| `server_port` | `integer\|null` | Default 20101 |
| `actual_timing_offset` | `integer\|null` | |
| `commanded_timing_offset` | `integer\|null` | |
| `correction_data` | `array\|null` | 257×257×2 nested array of `number\|null`; `null` = out-of-field (NaN in HDF5) |
| `inverse_correction_data` | `array\|null` | Same shape as `correction_data` |
| `manufacturer` | `string\|null` | |
| `model` | `string\|null` | |
| `output_path` | `string\|null` | |
| `selected_camera` | `string\|null` | |
| `custom_video_format` | `string\|null` | |
| `video_output` | `string\|null` | |
| `show_console` | `boolean\|null` | |
| `software_trigger_delay` | `integer\|null` | Milliseconds |
| `volts_to_watts_algorithm` | `string\|null` | |
| `volts_to_watts_params` | `string\|null` | |
| `correction_grid_domain_shape` | `string\|null` | |
| `inverse_grid_domain_shape` | `string\|null` | |

> By default `correction_data` and `inverse_correction_data` are omitted from `export-json`
> output (and therefore not present in the schema-validated JSON). They only appear when
> `include_binary=True` / `--include-binary` is used.

---

### scan_field_correction_file

Optional (`object | null`). Present when a ClearBox correction file is attached to the train.

| Field | Type | Notes |
|---|---|---|
| `document_name` | `string` | **Required** |
| `document_id` | `string` | **Required** — UUID string |
| `file_size` | `integer` | **Required** — byte size of the `.fc3` file |
| `valid_as_of_date` | `string` | **Required** — ISO 8601 date |
| `document_created_at` | `string\|null` | |
| `document_type` | `string\|null` | |
| `original_uri` | `string\|null` | |
| `raw_bytes` | `string\|null` | Base64-encoded `.fc3` file contents; omitted from default JSON output |

---

## opcua (non-schema, HDF5 only)

OPCUA telemetry configuration is stored in a separate `OPCUA/` HDF5 group tree and is **not
part of the JSON schema**. It does not appear in `export-json` output. It is accessed through
the model's `opcua` field (when populated) or via `get_raw_group()`.

**HDF5 group structure:**

```
OPCUA/
  Client/          attributes: Server_URL, Auth_Mode, Security_Mode, Security_Policy,
                               BFS_Max_Depth, Publish_Interval, Sampling_Interval,
                               Session_Timeout, + any extra attributes
  Pipe/            attributes: Pipe_Enabled (int 0/1), Buffer_Size, + extras
  Triggers/        attribute:  Triggers_Enabled (float64 0.0/1.0)
    <trigger_name>/  attributes: ID, Signal, Subsystem, Rule_Enabled (int 0/1),
                                 Start_Value, Stop_Value, + extras
```

All languages model this identically: `OpcuaConfig { client, pipe, triggers, triggers_enabled }`.
Unknown attributes on any group are collected into the `extra` map and round-tripped verbatim.

---

## Conventions

- **Unit locking (Rule 8)**: all `*_unit` fields have their values locked at parse time. A unit
  mismatch (e.g. reading a file where `working_distance_unit` is `"in"` instead of `"mm"`)
  raises an error rather than silently returning the wrong unit.
- **NaN → null**: `NaN` values in HDF5 float64 attributes become `null` in the JSON model and
  `None`/`nullopt`/`undefined` in language models. The schema allows `number | null` for all
  optional numeric fields.
- **`extra` fields**: attributes present in HDF5 that are not mapped to named schema fields are
  collected into an `extra` object on the relevant struct. They are written back verbatim by the
  writer. Values must be `string`, `integer`, or `number`.
- **Binary data is outside the schema**: `correction_data`, `inverse_correction_data`, and
  `raw_bytes` appear in the schema definition as optional but are excluded from the default
  `export-json` output. The schema therefore validates the default output; it does not constrain
  the full binary output.
