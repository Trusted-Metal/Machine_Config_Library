/**
 * TypeScript interfaces for the Machine Config Library.
 *
 * All field names are snake_case and match the canonical JSON output produced
 * by every language implementation (Python, Rust, Node.js).  These types are
 * derived directly from MachineConfigReader._config_to_dict() in the Python
 * reference implementation.
 */

import type { ExtensionTree } from "./extensions/contract.js";

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
   * files of either version. Omitted entirely (not `null`) when absent, which keeps v1.0 output
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
 * A structured algorithm + equation + constants + characterization points describing a power
 * conversion. `LightSource.power_characterization` uses it; ClearBox's own Power_Characterization
 * is an opaque extension tree and is not typed here.
 */
export interface PowerCharacterization {
  algorithm_type: string | null;
  algorithm_equation: string | null;
  input_type: string | null;
  units_derived_quantity: string | null;
  derivation_equation_constants: EquationConstant[];
  characterization_points: CalibrationPoint[];
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
  /**
   * Reader-computed, in-memory only: `true`/`false` once a
   * `MachineConfigReader` has recomputed the configuration hash and
   * compared it to the on-disk value; absent until then. Never set by a
   * writer or by `MockConfigBuilder`. Unconditionally excluded from JSON
   * output by the `omitFalseInvertFlags` replacer in each capability's
   * `hdf5.ts` — never written to HDF5, never part of the interop contract.
   * A mismatch is never an error: the file remains fully readable either
   * way (see `hash.ts`).
   */
  is_valid?: boolean;
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
  scan_field_correction_file: ScanFieldCorrectionFile | null;
}

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

export interface MachineConfig {
  meta: MachineConfigMeta;
  machine: Machine;
  optical_trains: OpticalTrain[];
  /**
   * Opaque extension trees keyed by their group name under `Extensions/`.
   * Read and written verbatim by core, so extensions core has no model for
   * (including private ones) survive a read-modify-write. Interpreted only by
   * the matching extension library. ClearBox is one of them (always in its File_Version 1.1
   * layout, for either file version).
   */
  extensions?: Record<string, ExtensionTree>;
}
