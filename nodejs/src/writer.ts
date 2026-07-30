import * as h5wasm from "h5wasm/node";
import { resolve } from "node:path";
import type {
  MachineConfig,
  OpticalTrain,
  Scanner,
  AxisConfig,
  LightSource,
  Collimator,
  ScannerCard,
  ClearBox,
  ScanFieldCorrectionFile,
  OpcuaConfig,
} from "./models.js";

// ---------------------------------------------------------------------------
// WASM init — lazy singleton (shared module-level state with reader.ts when
// both run in the same process).
// ---------------------------------------------------------------------------

let _readyPromise: Promise<void> | null = null;

function ensureReady(): Promise<void> {
  if (_readyPromise === null) {
    _readyPromise = h5wasm.ready.then(() => undefined);
  }
  return _readyPromise;
}

// ---------------------------------------------------------------------------
// Path helpers — NODERAWFS requires forward slashes on all platforms.
// ---------------------------------------------------------------------------

function normPath(p: string): string {
  return resolve(p).replace(/\\/g, "/");
}

// ---------------------------------------------------------------------------
// Write helpers — exact inverses of the reader helpers in reader.ts.
//
// Convention (mirrors Python _s/_f/_i/_b and Rust ws/wf/wi/wb):
//   null / undefined  →  "" (empty VarLen string attribute)
//   string            →  VarLen string attribute       dtype "S"
//   float             →  float64 attribute              dtype "<d"
//   integer           →  int64 attribute                dtype "<i8"
//   boolean           →  int64 (0 or 1)                 dtype "<i8"
//
// File extends Group in h5wasm, so all helpers that accept Group work on the
// File root object too.
// ---------------------------------------------------------------------------

function ws(grp: h5wasm.Group, key: string, val: string | null | undefined): void {
  grp.create_attribute(key, val ?? "", [], "S");
}

function wf(grp: h5wasm.Group, key: string, val: number | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, "", [], "S");
  } else {
    grp.create_attribute(key, val, [], "<d");
  }
}

function wi(grp: h5wasm.Group, key: string, val: number | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, "", [], "S");
  } else {
    grp.create_attribute(key, Math.trunc(val), [], "<i8");
  }
}

function wb(grp: h5wasm.Group, key: string, val: boolean | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, "", [], "S");
  } else {
    grp.create_attribute(key, val ? 1 : 0, [], "<i8");
  }
}

/** Write a string attribute on a Dataset (used for SFCF metadata). */
function ws_ds(ds: h5wasm.Dataset, key: string, val: string): void {
  ds.create_attribute(key, val, [], "S");
}

/** Write an int64 attribute on a Dataset (used for SFCF file_size). */
function wi_ds(ds: h5wasm.Dataset, key: string, val: number): void {
  ds.create_attribute(key, Math.trunc(val), [], "<i8");
}

/**
 * Write `extra` fields back as HDF5 attributes, preserving natural type.
 * Mirrors the Rust `write_extra_value` function.
 */
function writeExtra(grp: h5wasm.Group, extra: Record<string, unknown>): void {
  for (const [key, val] of Object.entries(extra)) {
    if (typeof val === "string") {
      grp.create_attribute(key, val, [], "S");
    } else if (typeof val === "number") {
      if (Number.isInteger(val)) {
        grp.create_attribute(key, val, [], "<i8");
      } else {
        grp.create_attribute(key, val, [], "<d");
      }
    } else if (typeof val === "boolean") {
      grp.create_attribute(key, val ? 1 : 0, [], "<i8");
    } else if (val != null) {
      grp.create_attribute(key, JSON.stringify(val), [], "S");
    }
  }
}

// ---------------------------------------------------------------------------
// Correction data helpers
// ---------------------------------------------------------------------------

const CORRECTION_SHAPE: [number, number, number] = [257, 257, 2];

/**
 * Convert a nested 3-D array (null cells → NaN) to a flat Float64Array.
 * Returns a zero-filled array of default shape when data is absent —
 * identical behaviour to the Python and Rust writers.
 */
function nestedToFlat(
  data: Array<Array<Array<number | null>>> | null | undefined,
): { flat: Float64Array; shape: [number, number, number] } {
  if (!data || data.length === 0) {
    const size = CORRECTION_SHAPE[0] * CORRECTION_SHAPE[1] * CORRECTION_SHAPE[2];
    return { flat: new Float64Array(size), shape: CORRECTION_SHAPE };
  }
  const d0 = data.length;
  const d1 = data[0]?.length ?? 0;
  const d2 = data[0]?.[0]?.length ?? 0;
  if (d1 === 0 || d2 === 0) {
    const size = CORRECTION_SHAPE[0] * CORRECTION_SHAPE[1] * CORRECTION_SHAPE[2];
    return { flat: new Float64Array(size), shape: CORRECTION_SHAPE };
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

/** Write a named Float64 3-D dataset with standard ClearBox attributes. */
function writeCorrectionDataset(
  grp: h5wasm.Group,
  name: string,
  data: Array<Array<Array<number | null>>> | null | undefined,
): void {
  const { flat, shape } = nestedToFlat(data);
  const ds = grp.create_dataset({ name, data: flat, shape, dtype: "<d" });
  ws_ds(ds, "dimensions", "H,W,D");
  ws_ds(ds, "dtype", "float64");
  ws_ds(ds, "shape", `${shape[0]}x${shape[1]}x${shape[2]}`);
}

// ---------------------------------------------------------------------------
// Sub-tree writers — one function per HDF5 group, mirroring reader.ts
// ---------------------------------------------------------------------------

function writeAxis(grp: h5wasm.Group, ax: AxisConfig | null): void {
  const a: AxisConfig = ax ?? {
    actual_bit_resolution: null,
    actual_bit_resolution_unit: null,
    commanded_bit_resolution: null,
    commanded_bit_resolution_unit: null,
    control_type: null,
    range_of_motion: null,
    range_of_motion_unit: null,
    smoothing_kernel: null,
    smoothing_parameters: null,
    tuning_parameters: null,
    tuning_type: null,
  };
  wi(grp, "Actual_Bit_Resolution", a.actual_bit_resolution);
  ws(grp, "Actual_Bit_Resolution_unit", a.actual_bit_resolution_unit);
  wi(grp, "Commanded_Bit_Resolution", a.commanded_bit_resolution);
  ws(grp, "Commanded_Bit_Resolution_unit", a.commanded_bit_resolution_unit);
  ws(grp, "Control_Type", a.control_type);
  wf(grp, "Range_Of_Motion", a.range_of_motion);
  ws(grp, "Range_Of_Motion_unit", a.range_of_motion_unit);
  ws(grp, "Smoothing_Kernel", a.smoothing_kernel);
  wf(grp, "Smoothing_Parameters", a.smoothing_parameters);
  ws(grp, "Tuning_Parameters", a.tuning_parameters);
  ws(grp, "Tuning_Type", a.tuning_type);
}

function writeScanner(grp: h5wasm.Group, s: Scanner): void {
  ws(grp, "Manufacturer", s.manufacturer);
  ws(grp, "Model", s.model);
  ws(grp, "Serial_Number", s.serial_number);
  wf(grp, "Working_Distance", s.working_distance);
  ws(grp, "Working_Distance_unit", s.working_distance_unit ?? "mm");
  wf(grp, "Scan_Field_Size_X", s.scan_field_x);
  ws(grp, "Scan_Field_Size_X_unit", s.scan_field_x_unit ?? "mm");
  wf(grp, "Scan_Field_Size_Y", s.scan_field_y);
  ws(grp, "Scan_Field_Size_Y_unit", s.scan_field_y_unit ?? "mm");
  wf(grp, "Scan_Field_Size_Z", s.scan_field_z);
  ws(grp, "Scan_Field_Size_Z_unit", s.scan_field_z_unit ?? "mm");
  wf(grp, "Scan_Head_Offset_X", s.scan_head_offset_x);
  ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit ?? "mm");
  wf(grp, "Scan_Head_Offset_Y", s.scan_head_offset_y);
  ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit ?? "mm");
  wf(grp, "Scan_Head_Offset_Z", s.scan_head_offset_z);
  ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit ?? "mm");
  wf(grp, "Scan_Head_Rotation", s.scan_head_rotation);
  ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit ?? "degrees");
  ws(grp, "Axis_Configuration", s.axis_configuration);
  // X and Y axes are always written (even if null — written with empty defaults).
  writeAxis(grp.create_group("X_Axis"), s.x_axis);
  writeAxis(grp.create_group("Y_Axis"), s.y_axis);
  if (s.z_axis != null) writeAxis(grp.create_group("Z_Axis"), s.z_axis);
  if (s.focus != null) writeAxis(grp.create_group("Focus"), s.focus);
}

function writeLightSource(grp: h5wasm.Group, ls: LightSource): void {
  ws(grp, "Manufacturer", ls.manufacturer);
  ws(grp, "Model", ls.model);
  ws(grp, "Serial_Number", ls.serial_number);
  wf(grp, "Light_Wavelength", ls.wavelength);
  ws(grp, "Light_Wavelength_unit", ls.wavelength_unit ?? "nm");
  wf(grp, "Power_Max_Nominal", ls.power_max_nominal);
  ws(grp, "Power_Max_Nominal_unit", ls.power_max_nominal_unit ?? "W");
  wf(grp, "Power_Max_Actual", ls.power_max_actual);
  ws(grp, "Power_Max_Actual_unit", ls.power_max_actual_unit ?? "W");
  wf(grp, "Power_Min_Actual", ls.power_min_actual);
  ws(grp, "Power_Min_Actual_unit", ls.power_min_actual_unit ?? "W");
  wf(grp, "Power_Min_Nominal", ls.power_min_nominal);
  ws(grp, "Power_Min_Nominal_unit", ls.power_min_nominal_unit ?? "W");
  // Power_Bit_Resolution is stored as a string in all real HDF5 files.
  ws(grp, "Power_Bit_Resolution",
    ls.power_bit_resolution != null ? String(ls.power_bit_resolution) : null);
  ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit ?? "bits");
  ws(grp, "Watts_To_Volts_Algorithm", ls.watts_to_volts_algorithm);
  ws(grp, "Watts_To_Volts_Params", ls.watts_to_volts_params);
}

function writeCollimator(grp: h5wasm.Group, c: Collimator): void {
  ws(grp, "Manufacturer", c.manufacturer);
  ws(grp, "Model", c.model);
  ws(grp, "Serial_Number", c.serial_number);
  wf(grp, "Focal_Length", c.focal_length);
  ws(grp, "Focal_Length_unit", c.focal_length_unit ?? "mm");
}

function writeScannerCard(grp: h5wasm.Group, sc: ScannerCard): void {
  ws(grp, "Manufacturer", sc.manufacturer);
  ws(grp, "Model", sc.model);
  ws(grp, "Serial_Number", sc.serial_number);
  ws(grp, "Communication_Protocol", sc.communication_protocol);
  wf(grp, "Sample_Period", sc.sample_period);
  ws(grp, "Sample_Period_unit", sc.sample_period_unit ?? "\u03bcs");
}

function writeClearBox(grp: h5wasm.Group, cb: ClearBox): void {
  ws(grp, "Ip_Address", cb.ip_address);
  ws(grp, "Serial_Number", cb.serial_number);
  wi(grp, "Data_Port", cb.data_port);
  wi(grp, "Server_Port", cb.server_port);
  wi(grp, "Actual_Timing_Offset", cb.actual_timing_offset);
  wi(grp, "Commanded_Timing_Offset", cb.commanded_timing_offset);
  ws(grp, "Manufacturer", cb.manufacturer);
  ws(grp, "Model", cb.model);
  ws(grp, "Output_Path", cb.output_path);
  ws(grp, "Selected_Camera", cb.selected_camera);
  ws(grp, "Custom_Video_Format", cb.custom_video_format);
  ws(grp, "Video_Output", cb.video_output);
  wb(grp, "Show_Console", cb.show_console);
  wi(grp, "Software_Trigger_Delay", cb.software_trigger_delay);
  ws(grp, "Volts_To_Watts_Algorithm", cb.volts_to_watts_algorithm);
  ws(grp, "Volts_To_Watts_Params", cb.volts_to_watts_params);
  ws(grp, "Correction_Grid_Domain_Shape", cb.correction_grid_domain_shape);
  ws(grp, "Inverse_Grid_Domain_Shape", cb.inverse_grid_domain_shape);
  writeCorrectionDataset(grp, "Correction_Data", cb.correction_data);
  writeCorrectionDataset(grp, "Inverse_Correction_Data", cb.inverse_correction_data);
}

function writeSfcf(trainGrp: h5wasm.Group, sfcf: ScanFieldCorrectionFile): void {
  let bytes: Uint8Array;
  if (sfcf.raw_bytes) {
    bytes = Buffer.from(sfcf.raw_bytes, "base64");
  } else {
    // Write zero-filled placeholder of the declared size (same as Python/Rust).
    const len = Math.max(sfcf.file_size ?? 1, 1);
    bytes = new Uint8Array(len);
  }
  const ds = trainGrp.create_dataset({
    name: "scan_field_correction_file",
    data: bytes,
    shape: [bytes.length],
    dtype: "<B",
  });
  ws_ds(ds, "document_name", sfcf.document_name);
  ws_ds(ds, "document_id", sfcf.document_id);
  wi_ds(ds, "file_size", sfcf.file_size);
  ws_ds(ds, "valid_as_of_date", sfcf.valid_as_of_date);
  ws_ds(ds, "document_created_at", sfcf.document_created_at ?? "");
  ws_ds(ds, "document_type", sfcf.document_type ?? "");
  ws_ds(ds, "original_uri", sfcf.original_uri ?? "");
}

function writeOpticalTrain(
  trainsGrp: h5wasm.Group,
  trainIdx: number,
  train: OpticalTrain,
): void {
  const tid = `Optical_Train_${String(trainIdx + 1).padStart(2, "0")}`;
  const trainGrp = trainsGrp.create_group(tid);

  ws(trainGrp, "ID", train.id);
  ws(trainGrp, "Beam_Profile_Type", train.beam_profile_type);
  ws(trainGrp, "Beam_Waist_Definition", train.beam_waist_definition);
  wf(trainGrp, "Beam_Waist_Major", train.beam_waist_major);
  ws(trainGrp, "Beam_Waist_Major_unit", train.beam_waist_major_unit ?? "\u03bcm");
  wf(trainGrp, "Beam_Waist_Minor", train.beam_waist_minor);
  ws(trainGrp, "Beam_Waist_Minor_unit", train.beam_waist_minor_unit ?? "\u03bcm");
  wf(trainGrp, "Beam_Waist_Offset_Z", train.beam_waist_offset_z);
  ws(trainGrp, "Beam_Waist_Offset_Z_unit", train.beam_waist_offset_z_unit ?? "mm");
  wf(trainGrp, "Build_Plane_Offset_Major", train.build_plane_offset_major);
  ws(trainGrp, "Build_Plane_Offset_Major_unit", train.build_plane_offset_major_unit ?? "mm");
  wf(trainGrp, "Build_Plane_Offset_Minor", train.build_plane_offset_minor);
  ws(trainGrp, "Build_Plane_Offset_Minor_unit", train.build_plane_offset_minor_unit ?? "mm");
  wf(trainGrp, "Collimator_Focal_Length", train.collimator_focal_length);
  ws(trainGrp, "Collimator_Focal_Length_unit", train.collimator_focal_length_unit ?? "mm");
  wf(trainGrp, "M2_Major", train.m2_major);
  wf(trainGrp, "M2_Minor", train.m2_minor);
  wf(trainGrp, "Major_Axis_Angle", train.major_axis_angle);
  ws(trainGrp, "Major_Axis_Angle_unit", train.major_axis_angle_unit ?? "degrees");
  wf(trainGrp, "Rayleigh_Length_Major", train.rayleigh_length_major);
  ws(trainGrp, "Rayleigh_Length_Major_unit", train.rayleigh_length_major_unit ?? "mm");
  wf(trainGrp, "Rayleigh_Length_Minor", train.rayleigh_length_minor);
  ws(trainGrp, "Rayleigh_Length_Minor_unit", train.rayleigh_length_minor_unit ?? "mm");
  ws(trainGrp, "Scanner_Number", train.scanner_number);
  wb(trainGrp, "Thermal_Lensing_Test_Passed", train.thermal_lensing_passed);
  wf(trainGrp, "Thermal_Lensing_Focal_Plane_Shift", train.thermal_lensing_focal_plane_shift);
  ws(trainGrp, "Thermal_Lensing_Focal_Plane_Shift_unit",
    train.thermal_lensing_focal_plane_shift_unit ?? "mm");
  wf(trainGrp, "Thermal_Lensing_Threshold", train.thermal_lensing_threshold);
  ws(trainGrp, "Thermal_Lensing_Threshold_unit", train.thermal_lensing_threshold_unit ?? "mm");

  writeScanner(trainGrp.create_group("Scanner"), train.scanner);
  writeLightSource(trainGrp.create_group("Light_Source"), train.light_source);
  writeCollimator(trainGrp.create_group("Collimator"), train.collimator);
  writeScannerCard(trainGrp.create_group("Scanner_Card"), train.scanner_card);

  if (train.optional_components.clearbox != null) {
    const optGrp = trainGrp.create_group("Optional_Components");
    writeClearBox(optGrp.create_group("ClearBox"), train.optional_components.clearbox);
  }

  if (train.scan_field_correction_file != null) {
    writeSfcf(trainGrp, train.scan_field_correction_file);
  }
}

function writeOpcua(f: h5wasm.File, opcua: OpcuaConfig): void {
  const opcuaGrp = f.create_group("OPCUA");

  const clientGrp = opcuaGrp.create_group("Client");
  const c = opcua.client;
  ws(clientGrp, "Server_URL", c.server_url);
  ws(clientGrp, "Auth_Mode", c.auth_mode);
  ws(clientGrp, "Security_Mode", c.security_mode);
  ws(clientGrp, "Security_Policy", c.security_policy);
  clientGrp.create_attribute("BFS_Max_Depth", c.bfs_max_depth, [], "<i8");
  clientGrp.create_attribute("Publish_Interval", c.publish_interval, [], "<i8");
  clientGrp.create_attribute("Sampling_Interval", c.sampling_interval, [], "<i8");
  clientGrp.create_attribute("Session_Timeout", c.session_timeout, [], "<i8");
  writeExtra(clientGrp, c.extra);

  const pipeGrp = opcuaGrp.create_group("Pipe");
  const p = opcua.pipe;
  pipeGrp.create_attribute("Pipe_Enabled", p.pipe_enabled ? 1 : 0, [], "<i8");
  pipeGrp.create_attribute("Buffer_Size", p.buffer_size, [], "<i8");
  writeExtra(pipeGrp, p.extra);

  const triggersGrp = opcuaGrp.create_group("Triggers");
  if (opcua.triggers_enabled != null) {
    // Triggers_Enabled is stored as float64 — matches Python (np.float64) and Rust writer.
    triggersGrp.create_attribute("Triggers_Enabled", opcua.triggers_enabled ? 1.0 : 0.0, [], "<d");
  }
  for (const [name, trigger] of Object.entries(opcua.triggers)) {
    const tGrp = triggersGrp.create_group(name);
    ws(tGrp, "ID", trigger.id);
    ws(tGrp, "Signal", trigger.signal);
    ws(tGrp, "Subsystem", trigger.subsystem);
    wb(tGrp, "Rule_Enabled", trigger.rule_enabled);
    ws(tGrp, "Start_Value", trigger.start_value);
    ws(tGrp, "Stop_Value", trigger.stop_value);
    writeExtra(tGrp, trigger.extra);
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Writes a MachineConfig to an HDF5 file.
 *
 * Exact inverse of MachineConfigReader.  Binary fields (correction_data,
 * inverse_correction_data, raw_bytes) are written as zero-filled arrays /
 * placeholders when absent from the config — identical to the Python and
 * Rust writer behaviour.
 */
export class MachineConfigWriter {
  constructor(private readonly config: MachineConfig) {}

  /**
   * Serialise config to a new HDF5 file at the given path (creates or overwrites).
   */
  async write(outputPath: string): Promise<void> {
    await ensureReady();
    const path = normPath(outputPath);
    const f = new h5wasm.File(path, "w");
    try {
      const meta = this.config.meta;
      // Root-level attributes (MachineConfigMeta)
      ws(f, "machine_name", meta.machine_name);
      ws(f, "manufacturer", meta.manufacturer);
      ws(f, "model", meta.model);
      ws(f, "serial_number", meta.serial_number);
      ws(f, "File_Version", meta.file_version);
      ws(f, "Export_Date", meta.export_date);
      ws(f, "Configuration_Hash", meta.configuration_hash);
      writeExtra(f, meta.extra);

      // Machine group
      const machineGrp = f.create_group("Machine");
      const ma = this.config.machine;
      ws(machineGrp, "ID", ma.id);
      ws(machineGrp, "Machine_Name", ma.machine_name);
      ws(machineGrp, "Manufacturer", ma.manufacturer);
      ws(machineGrp, "Model", ma.model);
      ws(machineGrp, "Serial_Number", ma.serial_number);
      wf(machineGrp, "Build_Plate_X_Dimension", ma.build_plate_x);
      ws(machineGrp, "Build_Plate_X_Dimension_unit", ma.build_plate_x_unit ?? "mm");
      wf(machineGrp, "Build_Plate_Y_Dimension", ma.build_plate_y);
      ws(machineGrp, "Build_Plate_Y_Dimension_unit", ma.build_plate_y_unit ?? "mm");
      wf(machineGrp, "Build_Plate_Z_Dimension", ma.build_plate_z);
      ws(machineGrp, "Build_Plate_Z_Dimension_unit", ma.build_plate_z_unit ?? "mm");
      wf(machineGrp, "Build_Plate_Corner_Radius", ma.build_plate_radius);
      ws(machineGrp, "Build_Plate_Corner_Radius_unit", ma.build_plate_radius_unit ?? "mm");
      ws(machineGrp, "Gas_Flow_Direction", ma.gas_flow_direction);
      ws(machineGrp, "Recoat_Direction", ma.recoat_direction);

      // Optical trains
      const trainsGrp = machineGrp.create_group("Optical_Trains");
      for (let i = 0; i < this.config.optical_trains.length; i++) {
        writeOpticalTrain(trainsGrp, i, this.config.optical_trains[i]);
      }

      // OPC-UA (optional)
      if (this.config.opcua != null) {
        writeOpcua(f, this.config.opcua);
      }
    } finally {
      f.close();
    }
  }
}
