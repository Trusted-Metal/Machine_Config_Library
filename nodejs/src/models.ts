/**
 * TypeScript interfaces for the Machine Config Library.
 *
 * All field names are snake_case and match the canonical JSON output produced
 * by every language implementation (Python, Rust, Node.js).  These types are
 * derived directly from MachineConfigReader._config_to_dict() in the Python
 * reference implementation.
 */

// ---------------------------------------------------------------------------
// Sub-component types
// ---------------------------------------------------------------------------

export interface AxisConfig {
  actual_bit_resolution: number | null;
  actual_bit_resolution_unit: string | null;
  commanded_bit_resolution: number | null;
  commanded_bit_resolution_unit: string | null;
  control_type: string | null;
  range_of_motion: number | null;
  range_of_motion_unit: string | null;
  smoothing_kernel: string | null;
  smoothing_parameters: number | null;
  tuning_parameters: string | null;
  tuning_type: string | null;
}

export interface Scanner {
  manufacturer: string;
  model: string;
  serial_number: string;
  working_distance: number | null;
  working_distance_unit: string | null;
  scan_field_x: number | null;
  scan_field_x_unit: string | null;
  scan_field_y: number | null;
  scan_field_y_unit: string | null;
  scan_field_z: number | null;
  scan_field_z_unit: string | null;
  scan_head_offset_x: number | null;
  scan_head_offset_x_unit: string | null;
  scan_head_offset_y: number | null;
  scan_head_offset_y_unit: string | null;
  scan_head_offset_z: number | null;
  scan_head_offset_z_unit: string | null;
  scan_head_rotation: number | null;
  scan_head_rotation_unit: string | null;
  axis_configuration: string | null;
  x_axis: AxisConfig | null;
  y_axis: AxisConfig | null;
  z_axis: AxisConfig | null;
  focus: AxisConfig | null;
  /**
   * Plain boolean, not `boolean | null` — deliberately different from every
   * other field on this interface. Always `true` or `false` at the API
   * layer (defaults to `false` whether the on-disk attribute is absent or
   * explicitly 0); `false` never appears in JSON output (stripped by a
   * `JSON.stringify` replacer in `toJson()`), only `true` does
   * (user-confirmed, 2026-08-21).
   */
  invert_actual_x: boolean;
  invert_actual_y: boolean;
  invert_commanded_x: boolean;
  invert_commanded_y: boolean;
}

export interface LightSource {
  manufacturer: string;
  model: string;
  serial_number: string;
  wavelength: number | null;
  wavelength_unit: string | null;
  power_max_nominal: number | null;
  power_max_nominal_unit: string | null;
  power_max_actual: number | null;
  power_max_actual_unit: string | null;
  power_min_actual: number | null;
  power_min_actual_unit: string | null;
  power_min_nominal: number | null;
  power_min_nominal_unit: string | null;
  power_bit_resolution: number | null;
  power_bit_resolution_unit: string | null;
  /** The only representation of the watts<->volts conversion concept, for
   * files of either version. Omitted entirely (not `null`) when absent —
   * same reasoning as `ClearBox.synchronous_sensors`: keeps v1.0 output
   * byte-identical to before this field existed. */
  power_characterization?: PowerCharacterization | null;
}

export interface Collimator {
  manufacturer: string;
  model: string;
  serial_number: string;
  focal_length: number | null;
  focal_length_unit: string | null;
}

export interface ScannerCard {
  manufacturer: string;
  model: string;
  serial_number: string;
  communication_protocol: string | null;
  sample_period: number | null;
  sample_period_unit: string | null;
}

/**
 * On-disk row for `Derivation_Equation_Constants` — a variable-row-count
 * compound dataset naming an equation's constants (e.g. `a`/`b` for
 * `LINEAR`, `c0`..`cN` for `POLYNOMIAL`). `name` is stored as a 64-byte
 * fixed-length UTF-8 string on disk — see SYNCHRONOUS_SENSOR_PLAN.md's
 * "Compound dataset string convention" for why (a cross-language HDF5
 * restriction on variable-length strings inside compound-type members).
 */
export interface EquationConstant {
  name: string;
  value: number;
}

/**
 * On-disk row for `Calibration_Points` — a variable-row-count compound
 * dataset of raw calibration pairs. `input_value` is in whatever unit
 * `SynchronousSensor.input_type` implies; `output_value` is in whatever unit
 * `SynchronousSensor.sensor_output_space` implies (no per-row unit tag).
 */
export interface CalibrationPoint {
  input_value: number;
  output_value: number;
}

/**
 * v1.1 addition (Changes 3/4): a structured algorithm + equation + constants
 * + characterization points describing a power conversion. Shared, identical
 * interface for both `ClearBox.power_characterization` (ClearBox's
 * Volts->Watts fit; migrated data has real `derivation_equation_constants`
 * but zero `characterization_points`) and `LightSource.power_characterization`
 * (Light_Source's Volts->Watts fit; migrated data is the inverse — zero
 * `derivation_equation_constants`, real `characterization_points`) — same
 * *kind* of thing at two different HDF5 paths, not the same instance. See
 * `docs/migrations/v1_0_to_v1_1.md` Changes 3/4 for the full derivation rules.
 */
export interface PowerCharacterization {
  algorithm_type: string | null;
  algorithm_equation: string | null;
  input_type: string | null;
  units_derived_quantity: string | null;
  derivation_equation_constants: EquationConstant[];
  characterization_points: CalibrationPoint[];
}

/**
 * A single Synchronous Sensor record. Map key (on `ClearBox.synchronous_sensors`)
 * is a free-form label chosen by the file's author — not required to equal
 * any attribute value inside the sensor's own group (same convention as
 * `OpcuaConfig.triggers`'s keys).
 */
export interface SynchronousSensor {
  enabled: boolean | null;
  sensor_name: string | null;
  sensor_output_range_low: number | null;
  sensor_output_range_high: number | null;
  sensor_output_space: string | null;
  sensor_model: string | null;
  sensor_manufacturer: string | null;
  sensor_scope: string | null;
  units_derived_quantity: string | null;
  port_id: number | null;
  sensor_type: string | null;
  input_type: string | null;
  algorithm_type: string | null;
  algorithm_equation: string | null;
  calibration_source: string | null;
  calibration_verified: boolean | null;
  sample_period: number | null;
  metadata: string | null;
  derivation_equation_constants: EquationConstant[];
  calibration_points: CalibrationPoint[];
}

/**
 * A raw ClearBox correction grid — flat, row-major, NaN preserved (not
 * JSON-safe). Lives at the model layer (not a version-specific adapter)
 * since the shape is part of the stable model, not an on-disk detail — the
 * same reasoning already applied to the null<->NaN conversion helpers below.
 */
export interface CorrectionData {
  data: Float64Array;
  shape: [number, number, number];
}

export interface ClearBox {
  ip_address: string;
  serial_number: string | null;
  data_port: number | null;
  server_port: number | null;
  actual_timing_offset: number | null;
  commanded_timing_offset: number | null;
  /** Only present when include_binary=true. */
  correction_data?: Array<Array<Array<number | null>>> | null;
  /** Only present when include_binary=true. */
  inverse_correction_data?: Array<Array<Array<number | null>>> | null;
  manufacturer: string | null;
  model: string | null;
  output_path: string | null;
  selected_camera: string | null;
  custom_video_format: string | null;
  video_output: string | null;
  show_console: boolean | null;
  software_trigger_delay: number | null;
  correction_grid_domain_shape: string | null;
  inverse_grid_domain_shape: string | null;
  /**
   * Omitted entirely (not `{}`) when there are no sensors — matches Rust's
   * `#[serde(skip_serializing_if = "IndexMap::is_empty")]` and Python's
   * `_clearbox_to_dict` choice to omit the key when empty, so every fixture
   * that doesn't use this feature stays byte-for-byte identical in JSON
   * shape to before it existed. Same optional-whole-feature shape as
   * `MachineConfig.opcua?`, not `OpcuaConfig.triggers` (which is always
   * present, even as `{}`) — deliberately different from that precedent.
   */
  synchronous_sensors?: Record<string, SynchronousSensor>;
  /** v1.1 addition (Change 1); omitted entirely (not `null`) when absent —
   * always true for v1.0 files, no on-disk source there. Same reasoning as
   * `synchronous_sensors` above: keeps v1.0 output byte-identical. */
  firmware_version?: string | null;
  /** The only representation of the volts<->watts conversion concept, for
   * files of either version. Omitted when absent, same reasoning as
   * `firmware_version` above. */
  power_characterization?: PowerCharacterization | null;
}

export interface OptionalComponents {
  clearbox: ClearBox | null;
}

// ---------------------------------------------------------------------------
// Correction grid conversion helpers
//
// Live at the model layer rather than in a version-specific adapter: the
// NaN<->null convention is part of ClearBox's own field shape
// (`Array<Array<Array<number | null>>>`), not an on-disk detail of any
// particular File_Version, so every adapter can share it.
// ---------------------------------------------------------------------------

const DEFAULT_CORRECTION_SHAPE: [number, number, number] = [257, 257, 2];

/**
 * Convert a flat Float64Array (from an HDF5 dataset) into a nested 3-D
 * JavaScript array, mapping IEEE-754 NaN -> null to produce valid JSON.
 */
export function float64ToNested3D(
  data: Float64Array,
  shape: number[],
): Array<Array<Array<number | null>>> {
  const [d0, d1, d2] = shape;
  const out: Array<Array<Array<number | null>>> = [];
  let offset = 0;
  for (let i = 0; i < d0; i++) {
    const row: Array<Array<number | null>> = [];
    for (let j = 0; j < d1; j++) {
      const cell: Array<number | null> = [];
      for (let k = 0; k < d2; k++) {
        const v = data[offset++];
        cell.push(isNaN(v) ? null : v);
      }
      row.push(cell);
    }
    out.push(row);
  }
  return out;
}

/**
 * Convert a nested 3-D array (null cells -> NaN) to a flat Float64Array.
 * Returns a zero-filled array of default shape when data is absent —
 * identical behaviour to the Python and Rust writers.
 */
export function nestedToFlat(
  data: Array<Array<Array<number | null>>> | null | undefined,
): { flat: Float64Array; shape: [number, number, number] } {
  if (!data || data.length === 0) {
    const size = DEFAULT_CORRECTION_SHAPE[0] * DEFAULT_CORRECTION_SHAPE[1] * DEFAULT_CORRECTION_SHAPE[2];
    return { flat: new Float64Array(size), shape: DEFAULT_CORRECTION_SHAPE };
  }
  const d0 = data.length;
  const d1 = data[0]?.length ?? 0;
  const d2 = data[0]?.[0]?.length ?? 0;
  if (d1 === 0 || d2 === 0) {
    const size = DEFAULT_CORRECTION_SHAPE[0] * DEFAULT_CORRECTION_SHAPE[1] * DEFAULT_CORRECTION_SHAPE[2];
    return { flat: new Float64Array(size), shape: DEFAULT_CORRECTION_SHAPE };
  }
  const flat = new Float64Array(d0 * d1 * d2);
  let offset = 0;
  for (let i = 0; i < d0; i++) {
    for (let j = 0; j < d1; j++) {
      for (let k = 0; k < d2; k++) {
        const v = data[i][j][k];
        flat[offset++] = v == null ? NaN : v;
      }
    }
  }
  return { flat, shape: [d0, d1, d2] };
}

export interface ScanFieldCorrectionFile {
  document_name: string;
  document_id: string;
  file_size: number;
  valid_as_of_date: string;
  document_created_at: string | null;
  document_type: string | null;
  original_uri: string | null;
  /** Only present when include_binary=true. Base64-encoded bytes. */
  raw_bytes?: string | null;
}

// ---------------------------------------------------------------------------
// Core schema types
// ---------------------------------------------------------------------------

export interface MachineConfigMeta {
  schema_version: string;
  machine_name: string;
  manufacturer: string;
  model: string;
  serial_number: string;
  file_version: string;
  export_date: string;
  configuration_hash: string;
  // TEST FIXTURE for the mock v1.1 adapter (docs/migrations/mock_v1_0_to_v1_1.md).
  // Not real schema fields — never wired into JSON export or the JSON schema.
  facility_id?: string | null;
  config_author?: string | null;
  extra: Record<string, unknown>;
}

/**
 * Machine object.  Build-plate dimensions are stored as flat keys
 * (build_plate_x, build_plate_y, etc.) — NOT in a nested sub-object.
 */
export interface Machine {
  id: string | null;
  machine_name: string;
  manufacturer: string;
  model: string;
  serial_number: string;
  build_plate_x: number | null;
  build_plate_x_unit: string | null;
  build_plate_y: number | null;
  build_plate_y_unit: string | null;
  build_plate_z: number | null;
  build_plate_z_unit: string | null;
  build_plate_radius: number | null;
  build_plate_radius_unit: string | null;
  gas_flow_direction: string | null;
  recoat_direction: string | null;
  recoater_blade_type: string | null;
}

export interface OpticalTrain {
  train_id: string;
  id: string | null;
  beam_profile_type: string | null;
  beam_waist_definition: string | null;
  beam_waist_major: number | null;
  beam_waist_major_unit: string | null;
  beam_waist_minor: number | null;
  beam_waist_minor_unit: string | null;
  beam_waist_offset_z: number | null;
  beam_waist_offset_z_unit: string | null;
  build_plane_offset_major: number | null;
  build_plane_offset_major_unit: string | null;
  build_plane_offset_minor: number | null;
  build_plane_offset_minor_unit: string | null;
  collimator_focal_length: number | null;
  collimator_focal_length_unit: string | null;
  m2_major: number | null;
  m2_minor: number | null;
  major_axis_angle: number | null;
  major_axis_angle_unit: string | null;
  rayleigh_length_major: number | null;
  rayleigh_length_major_unit: string | null;
  rayleigh_length_minor: number | null;
  rayleigh_length_minor_unit: string | null;
  scanner_number: string | null;
  thermal_lensing_passed: boolean | null;
  thermal_lensing_focal_plane_shift: number | null;
  thermal_lensing_focal_plane_shift_unit: string | null;
  thermal_lensing_threshold: number | null;
  thermal_lensing_threshold_unit: string | null;
  scanner: Scanner;
  light_source: LightSource;
  collimator: Collimator;
  scanner_card: ScannerCard;
  /** Always present; clearbox is null when no ClearBox is installed. */
  optional_components: OptionalComponents;
  scan_field_correction_file: ScanFieldCorrectionFile | null;
}

// ---------------------------------------------------------------------------
// OPC-UA types
// ---------------------------------------------------------------------------

export interface OpcuaClientConfig {
  server_url: string;
  auth_mode: string;
  security_mode: string;
  security_policy: string;
  bfs_max_depth: number;
  publish_interval: number;
  sampling_interval: number;
  session_timeout: number;
  keep_alive_count: number | null;
  lifetime_count: number | null;
  machine_profile: string | null;
  queue_policy: string | null;
  queue_size_data_change: number | null;
  queue_size_events: number | null;
  reconnect_interval: number | null;
  root_node: string | null;
  sync_loop_interval_initial: number | null;
  sync_loop_interval_settled: number | null;
  extra: Record<string, unknown>;
}

export interface OpcuaPipeConfig {
  pipe_enabled: boolean;
  buffer_size: number;
  configure_client: boolean | null;
  inbound_rate_limit: number | null;
  max_inbound_message_size: number | null;
  min_integrity_level: string | null;
  pipe_name: string | null;
  user_access_level: string | null;
  extra: Record<string, unknown>;
}

export interface OpcuaTrigger {
  id: string | null;
  signal: string | null;
  subsystem: string | null;
  rule_enabled: boolean | null;
  start_value: string | null;
  stop_value: string | null;
  case_sensitivity: string | null;
  component: string | null;
  cooldown_period: number | null;
  event: string | null;
  max_fires_per_job: number | null;
  trigger_label: string | null;
  extra: Record<string, unknown>;
}

export interface OpcuaConfig {
  client: OpcuaClientConfig;
  pipe: OpcuaPipeConfig;
  triggers_enabled: boolean | null;
  trigger_stop_ceiling_layers: number | null;
  triggers: Record<string, OpcuaTrigger>;
}

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

export interface MachineConfig {
  meta: MachineConfigMeta;
  machine: Machine;
  optical_trains: OpticalTrain[];
  /** OPC-UA configuration; omitted when no OPC-UA config is present. */
  opcua?: OpcuaConfig;
}
