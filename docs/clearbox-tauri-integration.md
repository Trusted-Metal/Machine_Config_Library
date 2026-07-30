# clearbox-tauri Integration Guide

**Status**: Library ready — 60/60 Rust tests pass, full cross-check passes (schema, read parity, write interop, correction data hashes).

This document describes how to replace `clearbox-tauri`'s own HDF5 reader
(`src-tauri/src/machine_config/`) with the `machine-config` Rust library,
and how each field in clearbox-tauri's types maps to the library's model.

---

## 1. Adding the Dependency

In `clearbox-tauri/src-tauri/Cargo.toml`, add:

```toml
[dependencies]
machine-config = { path = "../../Machine_Config_Library/rust" }
```

Adjust the relative path if the two repos are not siblings on disk.

### Version compatibility

| Crate | machine-config | clearbox-tauri | Compatible? |
|---|---|---|---|
| `hdf5-metno` | `0.12` | `0.12` | ✅ exact match |
| `serde` | `1` | `1` | ✅ |
| `serde_json` | `1` | `1` | ✅ |
| `ndarray` | `0.16` | `0.17.2` | ⚠️ see below |
| `thiserror` | `1` | `1` | ✅ |

**ndarray version**: The library uses `0.16` internally; clearbox-tauri uses `0.17.2`.
Both versions will compile into the binary (Cargo handles this as separate crates).
This is harmless because the library's **public API never exposes ndarray types** —
correction data is returned as a plain `Vec<f64>` + `[usize; 3]` shape
(see §5 below). No conversion or bridging is needed at the callsite.

---

## 2. Integration Strategy

**Recommended: Augment, don't replace (Phase 1)**

Keep `LaserConfig`, `SettingsConfig`, and the rest of clearbox-tauri's
existing types unchanged. Replace only the HDF5 *reading* code with calls to
the library, then map from library types to clearbox-tauri types.

This keeps the change localised to `src-tauri/src/machine_config/reader.rs`
and avoids touching every callsite that uses `LaserConfig`.

```rust
// src-tauri/src/machine_config/reader.rs  (new implementation)
use machine_config::reader::MachineConfigReader;
use machine_config::models::{MachineConfig as LibConfig, OpticalTrain};

pub fn read_machine_config(path: &Path) -> Result<MachineConfig, ClearBoxError> {
    let lib_cfg = MachineConfigReader::new(path)
        .parse_with_binary()          // loads correction grids too
        .map_err(|e| ClearBoxError::Io(...))?;

    Ok(map_to_clearbox_config(lib_cfg, path))
}
```

**Optional Phase 2**: Once the rest of the codebase is stable, migrate
callsites to use `machine_config::models::*` directly and delete the
redundant `LaserConfig` / `SettingsConfig` types.

---

## 3. Field Mapping: `SettingsConfig` ← library types

`SettingsConfig` comes from machine-level data in `MachineConfig`.

| `SettingsConfig` field | Library path | Notes |
|---|---|---|
| `machine_name` | `config.meta.machine_name` | also at `config.machine.machine_name` |
| `manufacturer` | `config.machine.manufacturer` | |
| `model` | `config.machine.model` | |
| `machine_serial` | `config.machine.serial_number` | |
| `communication_protocol` | `train.scanner_card.communication_protocol` | use train 0; `None` → `""` |
| `is_3d_system` | `train.scanner.axis_configuration == "3D" \|\| == "3D+Focus"` | use train 0 |
| `output_path` | `train.optional_components.clearbox?.output_path` | use train 0; `None` → `""` |
| `selected_camera` | `train.optional_components.clearbox?.selected_camera` | |
| `video_output` | `train.optional_components.clearbox?.video_output` | |
| `custom_video_format` | `train.optional_components.clearbox?.custom_video_format` | |
| `show_console` | `train.optional_components.clearbox?.show_console` | `None` → `false` |
| `recoater_blade_type` | *(not in library model — keep default `"Standard"`)* | |
| `st_delay` | *(not in library model — keep default `0.5`)* | |
| `double_file` | *(not in library model — keep default `false`)* | |

---

## 4. Field Mapping: `LaserConfig` ← `OpticalTrain`

One `LaserConfig` per `OpticalTrain`. `laser_num` = train index + 1.

### ClearBox fields (all under `train.optional_components.clearbox`)

| `LaserConfig` field | Library path | Notes |
|---|---|---|
| `ip_address` | `cb.ip_address` | required field on `ClearBox` |
| `data_port` | `cb.data_port` | `Option<i64>` → `u16`; `None` → `5000` |
| `server_port` | `cb.server_port` | `Option<i64>` → `u16`; `None` → `20101` |
| `laser_offset_actual` | `cb.actual_timing_offset` | `Option<i64>` → `f64`; `None` → `0.0` |
| `laser_offset_commanded` | `cb.commanded_timing_offset` | `Option<i64>` → `f64`; `None` → `0.0` |
| `volts_to_watts_algorithm` | `cb.volts_to_watts_algorithm` | `None` → `""` |
| `volts_to_watts_params` | `cb.volts_to_watts_params` | `None` → `""` |
| `software_trigger_delay` | `cb.software_trigger_delay` | `Option<i64>` → `f64`; `None` → `0.0` |
| `output_path` | `cb.output_path` | `None` → `""` |
| `has_correction_data` | `cb.correction_data.is_some()` | when `parse_with_binary()` used |
| `has_inverse_correction_data` | `cb.inverse_correction_data.is_some()` | |

### Scanner fields (under `train.scanner`)

| `LaserConfig` field | Library path | Notes |
|---|---|---|
| `scanner_id` | `train.scanner.serial_number` | |
| `scanner_field_size_x` | `train.scanner.scan_field_x` | `Option<f64>`; `None` → `500.0` |
| `scanner_field_size_y` | `train.scanner.scan_field_y` | `None` → `500.0` |
| `scanner_field_size_z` | `train.scanner.scan_field_z` | `None` → `0.0` |
| `scanner_offset_x` | `train.scanner.scan_head_offset_x` | `None` → `0.0` |
| `scanner_offset_y` | `train.scanner.scan_head_offset_y` | `None` → `0.0` |
| `scanner_offset_z` | `train.scanner.scan_head_offset_z` | `None` → `0.0` |
| `scanner_rotation` | `train.scanner.scan_head_rotation` | `None` → `0.0` |
| `scan_head_model` | `format!("{} {}", scanner.manufacturer, scanner.model)` | |
| `scanner_serial` | `train.scanner.serial_number` | same as `scanner_id` |
| `x_actual_bit_res` | `train.scanner.x_axis?.actual_bit_resolution` | `Option<i64>` → `i32` |
| `x_commanded_bit_res` | `train.scanner.x_axis?.commanded_bit_resolution` | |
| `y_actual_bit_res` | `train.scanner.y_axis?.actual_bit_resolution` | |
| `y_commanded_bit_res` | `train.scanner.y_axis?.commanded_bit_resolution` | |
| `z_actual_bit_res` | `train.scanner.z_axis?.actual_bit_resolution` | `None` → `0` |
| `z_commanded_bit_res` | `train.scanner.z_axis?.commanded_bit_resolution` | |
| `x_galvo_range_of_motion` | `train.scanner.x_axis?.range_of_motion` | |
| `y_galvo_range_of_motion` | `train.scanner.y_axis?.range_of_motion` | |
| `z_galvo_range_of_motion` | `train.scanner.z_axis?.range_of_motion` | |
| `x_galvo_tuning_type` | `train.scanner.x_axis?.tuning_type` | |
| `y_galvo_tuning_type` | `train.scanner.y_axis?.tuning_type` | |
| `z_galvo_tuning_type` | `train.scanner.z_axis?.tuning_type` | |
| `x_smoothing_kernel` | `train.scanner.x_axis?.smoothing_kernel` | |
| `y_smoothing_kernel` | `train.scanner.y_axis?.smoothing_kernel` | |
| `z_smoothing_kernel` | `train.scanner.z_axis?.smoothing_kernel` | |
| `x_smoothing_parameters` | `train.scanner.x_axis?.smoothing_parameters` | |
| `y_smoothing_parameters` | `train.scanner.y_axis?.smoothing_parameters` | |
| `z_smoothing_parameters` | `train.scanner.z_axis?.smoothing_parameters` | |
| `x_tuning_parameters` | `train.scanner.x_axis?.tuning_parameters` | |
| `y_tuning_parameters` | `train.scanner.y_axis?.tuning_parameters` | |
| `z_tuning_parameters` | `train.scanner.z_axis?.tuning_parameters` | |
| `actual_bit_res` | `train.scanner.x_axis?.actual_bit_resolution.to_string()` | legacy field |
| `commanded_bit_res` | `train.scanner.x_axis?.commanded_bit_resolution.to_string()` | |

### Light source fields (under `train.light_source`)

| `LaserConfig` field | Library path | Notes |
|---|---|---|
| `light_wavelength` | `train.light_source.wavelength` | `None` → `1070.0` |
| `light_wavelength_unit` | `train.light_source.wavelength_unit` | `None` → `"nm"` |
| `power_bit_resolution` | `train.light_source.power_bit_resolution` | `Option<f64>` → `i32` |
| `power_min_nominal` | `train.light_source.power_min_nominal` | `None` → `0.0` |
| `power_max_nominal` | `train.light_source.power_max_nominal` | `None` → `0.0` |
| `power_min_actual` | `train.light_source.power_min_actual` | `None` → `0.0` |
| `power_max_actual` | `train.light_source.power_max_actual` | `None` → `0.0` |
| `laser_model` | `format!("{} {}", ls.manufacturer, ls.model)` | |
| `laser_serial_number` | `train.light_source.serial_number` | |

### Optical-train-level fields

| `LaserConfig` field | Library path | Notes |
|---|---|---|
| `communication_protocol` | `train.scanner_card.communication_protocol` | `None` → `""` |
| `second_moment_minor_axis_at_waist` | `train.beam_waist_minor` | `None` → `0.0` |
| `second_moment_major_axis_at_waist` | `train.beam_waist_major` | `None` → `0.0` |
| `second_moment_minor_at_build_plate` | `train.build_plane_offset_minor` | `None` → `0.0` |
| `second_moment_major_at_build_plate` | `train.build_plane_offset_major` | `None` → `0.0` |
| `rayleigh_length_minor_axis` | `train.rayleigh_length_minor` | `None` → `0.0` |
| `rayleigh_length_major_axis` | `train.rayleigh_length_major` | `None` → `0.0` |
| `build_plane_offset_x` | `train.build_plane_offset_major` | `None` → `0.0` |
| `build_plane_offset_y` | `train.build_plane_offset_minor` | `None` → `0.0` |
| `thermal_lensing_focus_shift` | `train.thermal_lensing_focal_plane_shift` | `None` → `0.0` |
| `major_axis_angle` | `train.major_axis_angle` | `None` → `0.0` |
| `invert_actual_x` | *(not in library model — keep default `false`)* | |
| `invert_actual_y` | *(not in library model — keep default `false`)* | |
| `invert_commanded_x` | *(not in library model — keep default `false`)* | |
| `invert_commanded_y` | *(not in library model — keep default `false`)* | |

### Axis inversion flags
These attributes do not exist in the current HDF5 schema and are not in the
library model. Retain defaults (`false`) until the schema is extended.

---

## 5. Correction Data

The library exposes correction data in two forms:

### 5a. Model form (nested, NaN→None)
`parse()` returns `cb.correction_data: Option<Vec<Vec<Vec<Option<f64>>>>>`.
Available only when `parse_with_binary()` is called.
Shape is always `[257][257][2]` — index `[i][j][0]` = X correction, `[i][j][1]` = Y correction.

### 5b. Raw form (flat buffer)
`reader.get_correction_data(train_index)` returns `CorrectionData { data: Vec<f64>, shape: [usize; 3] }`.
`data` is a flat, row-major buffer. Element at `(i, j, k)`:
```rust
data[i * shape[1] * shape[2] + j * shape[2] + k]
```
`NaN` values represent out-of-field cells.

### Mapping to clearbox-tauri `LaserConfig`

```rust
// Using the raw form — direct fit to LaserConfig's fields
let cd = reader.get_correction_data(train_index)?;
laser.correction_data      = Some(cd.data);
laser.correction_data_dims = Some(cd.shape);

let icd = reader.get_inverse_correction_data(train_index)?;
laser.inverse_correction_data      = Some(icd.data);
laser.inverse_correction_data_dims = Some(icd.shape);
```

### Mapping to `GalvoCorrection` (Vec<Vec<i32>>)

`GalvoCorrection` uses 257×257 `i32` tables. Convert from the library's
flat `f64` buffer:

```rust
// data: flat [257 * 257 * 2] f64 buffer, shape [257, 257, 2]
fn flat_f64_to_i32_table(data: &[f64], shape: [usize; 3])
    -> (Vec<Vec<i32>>, Vec<Vec<i32>>)
{
    let [d0, d1, _d2] = shape;
    let mut x_table = vec![vec![0i32; d1]; d0];
    let mut y_table = vec![vec![0i32; d1]; d0];
    for i in 0..d0 {
        for j in 0..d1 {
            let base = i * d1 * 2 + j * 2;
            x_table[i][j] = data[base].round() as i32;
            y_table[i][j] = data[base + 1].round() as i32;
        }
    }
    (x_table, y_table)
}
```

This also lets `load_correction_hdf5` be fully implemented by delegating to
the library instead of staying as a TODO stub:

```rust
// correction/loader.rs
pub fn load_correction_hdf5(
    path: &Path,
    train_index: usize,
) -> Result<(Vec<Vec<i32>>, Vec<Vec<i32>>), ClearBoxError> {
    use machine_config::reader::MachineConfigReader;
    let reader = MachineConfigReader::new(path)
        .map_err(|e| ClearBoxError::Correction(e.to_string()))?;
    let cd = reader.get_correction_data(train_index)
        .map_err(|e| ClearBoxError::Correction(e.to_string()))?;
    Ok(flat_f64_to_i32_table(&cd.data, cd.shape))
}
```

---

## 6. Configuration Hash Validation

The library exposes the SHA-256 hash stored in the HDF5 file as
`config.meta.configuration_hash`. Use it to confirm a loaded file has not
been corrupted:

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::new(path)?;
let config = reader.parse()?;
let stored_hash = &config.meta.configuration_hash;

// The reader also provides a recomputed hash (covers all non-hash datasets):
let recomputed = reader.compute_hash()?;
assert_eq!(stored_hash, &recomputed, "machine config file is corrupt");
```

---

## 7. OPC-UA Config Mapping

The library parses OPC-UA config when present:
`config.opcua: Option<machine_config::models::OpcuaConfig>`

Mapping to clearbox-tauri's `OpcuaConfig` (feature `opcua-client`):

| clearbox-tauri field | Library path |
|---|---|
| `server_url` | `opcua.client.server_url` |
| `auth_mode` | `opcua.client.auth_mode` |
| `security_mode` | `opcua.client.security_mode` |
| `security_policy` | `opcua.client.security_policy` |
| `bfs_max_depth` | `opcua.client.bfs_max_depth` |
| `publish_interval` | `opcua.client.publish_interval` |
| `sampling_interval` | `opcua.client.sampling_interval` |
| `session_timeout` | `opcua.client.session_timeout` |
| `pipe_enabled` | `opcua.pipe.pipe_enabled` |
| `buffer_size` | `opcua.pipe.buffer_size` |
| `triggers_enabled` | `opcua.triggers_enabled` |

**Triggers** are stored as `IndexMap<String, OpcuaTrigger>` keyed by the
trigger label (HDF5 sub-group name), mapping naturally to clearbox-tauri's
`Vec<TriggerConfig>` (use `trigger_label = key`, etc.).

---

## 8. What the Library Does NOT Cover

These fields exist in clearbox-tauri but have no counterpart in the library
model — keep their existing defaults or derive them from other sources:

- `LaserConfig::enabled` — clearbox-tauri business logic, not in HDF5 schema
- `LaserConfig::invert_actual_x/y`, `invert_commanded_x/y` — not in schema
- `SettingsConfig::recoater_blade_type`, `st_delay`, `double_file` — not in schema
- `BuildConfig` — legacy group; the library does not parse `/Configuration/build_configuration`
- `SensorConfig` — not in library model (different HDF5 group)
- `SettingsConfig::is_3d_system` — derive from `scanner.axis_configuration`

---

## 9. Public API Surface (library)

```rust
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;
use machine_config::models::{
    MachineConfig, MachineConfigMeta, Machine,
    OpticalTrain, OptionalComponents,
    ClearBox, CorrectionData, Scanner, AxisConfig,
    LightSource, Collimator, ScannerCard,
    ScanFieldCorrectionFile,
    OpcuaConfig, OpcuaClientConfig, OpcuaPipeConfig, OpcuaTrigger,
};
use machine_config::error::MachineConfigError;

// Read
let reader = MachineConfigReader::new("path/to/config.h5")?;
let config  = reader.parse()?;              // scalars only (fast)
let config  = reader.parse_with_binary()?;  // includes correction grids + fc3 bytes

// Correction data (raw form, avoids cloning the full model)
let cd  = reader.get_correction_data(0)?;         // train 0
let icd = reader.get_inverse_correction_data(0)?;

// JSON export
let json = reader.to_json(true, false)?;    // pretty=true, include_binary=false

// Write
MachineConfigWriter::new(config).write("output.h5")?;
```

---

## 10. Suggested Migration Sequence

1. Add `machine-config` path dependency to `src-tauri/Cargo.toml`
2. Confirm `cargo build` succeeds (ndarray dual-version is harmless)
3. Replace `src-tauri/src/machine_config/reader.rs`:
   - Call `MachineConfigReader::new(path)?.parse_with_binary()`
   - Map library types → existing `LaserConfig` / `SettingsConfig` using §3–4
4. Implement `load_correction_hdf5` in `correction/loader.rs` using §5
5. Implement config hash validation at load time using §6
6. Run clearbox-tauri test suite; fix any type mismatches
7. (Optional Phase 2) Replace `LaserConfig` / `SettingsConfig` with direct
   use of `machine_config::models::*` and remove the mapping layer
