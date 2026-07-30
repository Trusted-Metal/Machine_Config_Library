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
  watts_to_volts_algorithm: string | null;
  watts_to_volts_params: string | null;
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
  volts_to_watts_algorithm: string | null;
  volts_to_watts_params: string | null;
  correction_grid_domain_shape: string | null;
  inverse_grid_domain_shape: string | null;
}

export interface OptionalComponents {
  clearbox: ClearBox | null;
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
  extra: Record<string, unknown>;
}

export interface OpcuaPipeConfig {
  pipe_enabled: boolean;
  buffer_size: number;
  extra: Record<string, unknown>;
}

export interface OpcuaTrigger {
  id: string | null;
  signal: string | null;
  subsystem: string | null;
  rule_enabled: boolean | null;
  start_value: string | null;
  stop_value: string | null;
  extra: Record<string, unknown>;
}

export interface OpcuaConfig {
  client: OpcuaClientConfig;
  pipe: OpcuaPipeConfig;
  triggers_enabled: boolean | null;
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
