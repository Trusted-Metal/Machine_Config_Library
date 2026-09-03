/**
 * File_Version 1.1 HDF5 writer — on-disk layout, attribute names, and casting.
 * Public MachineConfigWriter dispatches here.
 *
 * Deliberately independent of the previous version's writer — see the
 * matching note in this directory's `hdf5.ts` module docstring.
 */
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
  SynchronousSensor,
  PowerCharacterization,
  EquationConstant,
  CalibrationPoint,
} from "../../models.js";
import { nestedToFlat } from "../../models.js";
import * as layout from "./layout.js";

// ---------------------------------------------------------------------------
// WASM init — lazy singleton, module-local.
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
// Write helpers — exact inverses of the reader helpers in this directory's
// hdf5.ts, independently defined (see that module's docstring).
//
// Convention:
//   null / undefined  →  "" (empty VarLen string attribute)
//   string            →  VarLen string attribute       dtype "S"
//   float             →  float64 attribute              dtype "<d"
//   integer           →  int64 attribute                dtype "<i8"
//   boolean           →  int64 (0 or 1)                 dtype "<i8"
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

/**
 * Writes an int64 attribute (`1`) only when `val` is `true`; writes nothing
 * at all when `false`. Used for Scanner's four `invert_*` fields.
 */
function wbIfTrue(grp: h5wasm.Group, key: string, val: boolean): void {
  if (val) {
    grp.create_attribute(key, 1, [], "<i8");
  }
}

function ws_ds(ds: h5wasm.Dataset, key: string, val: string): void {
  ds.create_attribute(key, val, [], "S");
}

function wi_ds(ds: h5wasm.Dataset, key: string, val: number): void {
  ds.create_attribute(key, Math.trunc(val), [], "<i8");
}

/** Write `extra` fields back as HDF5 attributes, preserving natural type. */
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
// Compound-dataset helpers (EquationConstant / CalibrationPoint rows) —
// shared shape between SynchronousSensor and Power_Characterization, but at
// different dataset names, so the name is a parameter rather than hardcoded.
//
// Derivation_Equation_Constants.name is a 64-byte fixed-length UTF-8 string
// on disk, not h5wasm's default variable-length string — the HDF5 C library
// can't convert between fixed- and variable-length strings inside
// compound-type members, and h5wasm can't write a non-empty VLEN string
// there at all. h5wasm silently truncates a too-long fixed-length string
// rather than erroring, so this writer rejects an oversized name explicitly
// instead.
// ---------------------------------------------------------------------------

const EQUATION_CONSTANT_NAME_MAX_BYTES = 64;

function writeEquationConstants(
  grp: h5wasm.Group,
  rows: EquationConstant[],
  datasetName: string,
): void {
  for (const c of rows) {
    const nameBytes = Buffer.byteLength(c.name, "utf-8");
    if (nameBytes > EQUATION_CONSTANT_NAME_MAX_BYTES) {
      throw new Error(
        `${datasetName} name ${JSON.stringify(c.name)} is ${nameBytes} UTF-8 ` +
          `bytes, which does not fit in the ${EQUATION_CONSTANT_NAME_MAX_BYTES}-byte ` +
          "fixed-length field (would otherwise be silently truncated on write).",
      );
    }
  }
  const data = new Map<string, unknown>([
    ["name", rows.map((c) => c.name)],
    ["value", new Float64Array(rows.map((c) => c.value))],
  ]);
  grp.create_dataset({
    name: datasetName,
    data,
    dtype: [["name", `S${EQUATION_CONSTANT_NAME_MAX_BYTES}`], ["value", "<d"]],
    shape: [rows.length],
  });
}

/** All-`f64`, no string member — unaffected by the fixed-length decision above. */
function writePointPairs(
  grp: h5wasm.Group,
  rows: CalibrationPoint[],
  datasetName: string,
): void {
  const data = new Map<string, unknown>([
    ["input_value", new Float64Array(rows.map((p) => p.input_value))],
    ["output_value", new Float64Array(rows.map((p) => p.output_value))],
  ]);
  grp.create_dataset({
    name: datasetName,
    data,
    dtype: [["input_value", "<d"], ["output_value", "<d"]],
    shape: [rows.length],
  });
}

function writeSynchronousSensor(grp: h5wasm.Group, sensor: SynchronousSensor): void {
  wb(grp, "Enabled", sensor.enabled);
  ws(grp, "Sensor_Name", sensor.sensor_name);
  wf(grp, "Sensor_Output_Range_Low", sensor.sensor_output_range_low);
  wf(grp, "Sensor_Output_Range_High", sensor.sensor_output_range_high);
  ws(grp, "Sensor_Output_Space", sensor.sensor_output_space);
  ws(grp, "Sensor_Model", sensor.sensor_model);
  ws(grp, "Sensor_Manufacturer", sensor.sensor_manufacturer);
  ws(grp, "Sensor_Scope", sensor.sensor_scope);
  ws(grp, "Units_Derived_Quantity", sensor.units_derived_quantity);
  wi(grp, "Port_ID", sensor.port_id);
  ws(grp, "Sensor_Type", sensor.sensor_type);
  ws(grp, "Input_Type", sensor.input_type);
  ws(grp, "Algorithm_Type", sensor.algorithm_type);
  ws(grp, "Algorithm_Equation", sensor.algorithm_equation);
  ws(grp, "Calibration_Source", sensor.calibration_source);
  wb(grp, "Calibration_Verified", sensor.calibration_verified);
  wf(grp, "Sample_Period", sensor.sample_period);
  ws(grp, "Metadata", sensor.metadata);
  writeEquationConstants(grp, sensor.derivation_equation_constants, "Derivation_Equation_Constants");
  writePointPairs(grp, sensor.calibration_points, "Calibration_Points");
}

/**
 * Changes 3/4: always writes a complete, present group — schema-complete
 * over omitted, even when `pc` is `null`/`undefined` or its lists are empty.
 */
function writePowerCharacterization(
  grp: h5wasm.Group,
  pc: PowerCharacterization | null | undefined,
): void {
  ws(grp, "Algorithm_Type", pc?.algorithm_type ?? null);
  ws(grp, "Algorithm_Equation", pc?.algorithm_equation ?? null);
  ws(grp, "Input_Type", pc?.input_type ?? null);
  ws(grp, "Units_Derived_Quantity", pc?.units_derived_quantity ?? null);
  writeEquationConstants(
    grp,
    pc?.derivation_equation_constants ?? [],
    layout.DS_DERIVATION_EQUATION_CONSTANTS,
  );
  writePointPairs(grp, pc?.characterization_points ?? [], layout.DS_CHARACTERIZATION_POINTS);
}

// ---------------------------------------------------------------------------
// Sub-tree writers — one function per HDF5 group
// ---------------------------------------------------------------------------

/** Change 5: Tuning_Parameters/Tuning_Type are never written, regardless of
 * what's on the model. */
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
  wbIfTrue(grp, "Invert_Actual_X", s.invert_actual_x);
  wbIfTrue(grp, "Invert_Actual_Y", s.invert_actual_y);
  wbIfTrue(grp, "Invert_Commanded_X", s.invert_commanded_x);
  wbIfTrue(grp, "Invert_Commanded_Y", s.invert_commanded_y);
  writeAxis(grp.create_group("X_Axis"), s.x_axis);
  writeAxis(grp.create_group("Y_Axis"), s.y_axis);
  if (s.z_axis != null) writeAxis(grp.create_group("Z_Axis"), s.z_axis);
  if (s.focus != null) writeAxis(grp.create_group("Focus"), s.focus);
}

/** Change 4: Watts_To_Volts_Algorithm/Params are superseded by
 * Power_Characterization and never written. */
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
  ws(grp, "Power_Bit_Resolution",
    ls.power_bit_resolution != null ? String(ls.power_bit_resolution) : null);
  ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit ?? "bits");
  writePowerCharacterization(grp.create_group(layout.GROUP_POWER_CHARACTERIZATION), ls.power_characterization);
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
  ws(grp, "Sample_Period_unit", sc.sample_period_unit ?? "μs");
}

/**
 * Writes only the per-train subset — Output_Path/Software_Trigger_Delay are
 * written once, at Extensions/ClearBox/ itself, by the caller before this is
 * invoked (Change 1's Consolidate). `grp` is already positioned at
 * `Extensions/ClearBox/<train_id>/`.
 */
function writeClearBox(grp: h5wasm.Group, cb: ClearBox): void {
  ws(grp, "Ip_Address", cb.ip_address);
  ws(grp, "Serial_Number", cb.serial_number);
  wi(grp, "Data_Port", cb.data_port);
  wi(grp, "Server_Port", cb.server_port);
  wi(grp, "Actual_Timing_Offset", cb.actual_timing_offset);
  wi(grp, "Commanded_Timing_Offset", cb.commanded_timing_offset);
  ws(grp, "Manufacturer", cb.manufacturer);
  ws(grp, "Model", cb.model);
  ws(grp, "Firmware_Version", cb.firmware_version ?? null);
  writeCorrectionDataset(grp, layout.DS_CORRECTION_DATA, cb.correction_data);
  writeCorrectionDataset(grp, layout.DS_INVERSE_CORRECTION_DATA, cb.inverse_correction_data);

  const sensors = cb.synchronous_sensors ?? {};
  const sensorNames = Object.keys(sensors);
  if (sensorNames.length > 0) {
    const sensorsGrp = grp.create_group("Synchronous_Sensors");
    for (const name of sensorNames) {
      writeSynchronousSensor(sensorsGrp.create_group(name), sensors[name]);
    }
  }

  writePowerCharacterization(grp.create_group(layout.GROUP_POWER_CHARACTERIZATION), cb.power_characterization);
}

function writeSfcf(trainGrp: h5wasm.Group, sfcf: ScanFieldCorrectionFile): void {
  let bytes: Uint8Array;
  if (sfcf.raw_bytes) {
    bytes = Buffer.from(sfcf.raw_bytes, "base64");
  } else {
    const len = Math.max(sfcf.file_size ?? 1, 1);
    bytes = new Uint8Array(len);
  }
  const ds = trainGrp.create_dataset({
    name: layout.DS_SCAN_FIELD_CORRECTION_FILE,
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

function writeTrainAttrs(grp: h5wasm.Group, train: OpticalTrain): void {
  ws(grp, "ID", train.id);
  ws(grp, "Beam_Profile_Type", train.beam_profile_type);
  ws(grp, "Beam_Waist_Definition", train.beam_waist_definition);
  wf(grp, "Beam_Waist_Major", train.beam_waist_major);
  ws(grp, "Beam_Waist_Major_unit", train.beam_waist_major_unit ?? "μm");
  wf(grp, "Beam_Waist_Minor", train.beam_waist_minor);
  ws(grp, "Beam_Waist_Minor_unit", train.beam_waist_minor_unit ?? "μm");
  wf(grp, "Beam_Waist_Offset_Z", train.beam_waist_offset_z);
  ws(grp, "Beam_Waist_Offset_Z_unit", train.beam_waist_offset_z_unit ?? "mm");
  wf(grp, "Build_Plane_Offset_Major", train.build_plane_offset_major);
  ws(grp, "Build_Plane_Offset_Major_unit", train.build_plane_offset_major_unit ?? "mm");
  wf(grp, "Build_Plane_Offset_Minor", train.build_plane_offset_minor);
  ws(grp, "Build_Plane_Offset_Minor_unit", train.build_plane_offset_minor_unit ?? "mm");
  wf(grp, "Collimator_Focal_Length", train.collimator_focal_length);
  ws(grp, "Collimator_Focal_Length_unit", train.collimator_focal_length_unit ?? "mm");
  wf(grp, "M2_Major", train.m2_major);
  wf(grp, "M2_Minor", train.m2_minor);
  wf(grp, "Major_Axis_Angle", train.major_axis_angle);
  ws(grp, "Major_Axis_Angle_unit", train.major_axis_angle_unit ?? "degrees");
  wf(grp, "Rayleigh_Length_Major", train.rayleigh_length_major);
  ws(grp, "Rayleigh_Length_Major_unit", train.rayleigh_length_major_unit ?? "mm");
  wf(grp, "Rayleigh_Length_Minor", train.rayleigh_length_minor);
  ws(grp, "Rayleigh_Length_Minor_unit", train.rayleigh_length_minor_unit ?? "mm");
  ws(grp, "Scanner_Number", train.scanner_number);
  wb(grp, "Thermal_Lensing_Test_Passed", train.thermal_lensing_passed);
  wf(grp, "Thermal_Lensing_Focal_Plane_Shift", train.thermal_lensing_focal_plane_shift);
  ws(grp, "Thermal_Lensing_Focal_Plane_Shift_unit",
    train.thermal_lensing_focal_plane_shift_unit ?? "mm");
  wf(grp, "Thermal_Lensing_Threshold", train.thermal_lensing_threshold);
  ws(grp, "Thermal_Lensing_Threshold_unit", train.thermal_lensing_threshold_unit ?? "mm");
}

function writeOpcua(f: h5wasm.File, opcua: OpcuaConfig): void {
  const extensionsGrp = f.get(layout.ROOT_EXTENSIONS);
  const opcuaGrp =
    extensionsGrp instanceof h5wasm.Group
      ? extensionsGrp.create_group("TM_OPCUA")
      : f.create_group(layout.ROOT_EXTENSIONS).create_group("TM_OPCUA");

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
  wi(clientGrp, "Keep_Alive_Count", c.keep_alive_count);
  wi(clientGrp, "Lifetime_Count", c.lifetime_count);
  ws(clientGrp, "Machine_Profile", c.machine_profile);
  ws(clientGrp, "Queue_Policy", c.queue_policy);
  wi(clientGrp, "Queue_Size_Data_Change", c.queue_size_data_change);
  wi(clientGrp, "Queue_Size_Events", c.queue_size_events);
  wi(clientGrp, "Reconnect_Interval", c.reconnect_interval);
  ws(clientGrp, "Root_Node", c.root_node);
  wi(clientGrp, "Sync_Loop_Interval_Initial", c.sync_loop_interval_initial);
  wi(clientGrp, "Sync_Loop_Interval_Settled", c.sync_loop_interval_settled);
  writeExtra(clientGrp, c.extra);

  const pipeGrp = opcuaGrp.create_group("Pipe");
  const p = opcua.pipe;
  pipeGrp.create_attribute("Pipe_Enabled", p.pipe_enabled ? 1 : 0, [], "<i8");
  pipeGrp.create_attribute("Buffer_Size", p.buffer_size, [], "<i8");
  wb(pipeGrp, "Configure_Client", p.configure_client);
  wi(pipeGrp, "Inbound_Rate_Limit", p.inbound_rate_limit);
  wi(pipeGrp, "Max_Inbound_Message_Size", p.max_inbound_message_size);
  ws(pipeGrp, "Min_Integrity_Level", p.min_integrity_level);
  ws(pipeGrp, "Pipe_Name", p.pipe_name);
  ws(pipeGrp, "User_Access_Level", p.user_access_level);
  writeExtra(pipeGrp, p.extra);

  const triggersGrp = opcuaGrp.create_group("Triggers");
  if (opcua.triggers_enabled != null) {
    triggersGrp.create_attribute("Triggers_Enabled", opcua.triggers_enabled ? 1.0 : 0.0, [], "<d");
  }
  wi(triggersGrp, "Trigger_Stop_Ceiling_Layers", opcua.trigger_stop_ceiling_layers);
  for (const [name, trigger] of Object.entries(opcua.triggers)) {
    const tGrp = triggersGrp.create_group(name);
    ws(tGrp, "ID", trigger.id);
    ws(tGrp, "Signal", trigger.signal);
    ws(tGrp, "Subsystem", trigger.subsystem);
    wb(tGrp, "Rule_Enabled", trigger.rule_enabled);
    ws(tGrp, "Start_Value", trigger.start_value);
    ws(tGrp, "Stop_Value", trigger.stop_value);
    ws(tGrp, "Case_Sensitivity", trigger.case_sensitivity);
    ws(tGrp, "Component", trigger.component);
    wi(tGrp, "Cooldown_Period", trigger.cooldown_period);
    ws(tGrp, "Event", trigger.event);
    wi(tGrp, "Max_Fires_Per_Job", trigger.max_fires_per_job);
    ws(tGrp, "Trigger_Label", trigger.trigger_label);
    writeExtra(tGrp, trigger.extra);
  }
}

// ---------------------------------------------------------------------------
// Change 1: Consolidate (Output_Path / Software_Trigger_Delay)
// ---------------------------------------------------------------------------

interface TrainClearbox {
  tid: string;
  cb: ClearBox;
}

/**
 * Collects `field` (via `getter`) from every ClearBox-bearing train; hard
 * error naming every train/value if they disagree. See the v1.0-to-v1.1
 * migration manifest's Change 1 "Consolidate conflict rule" for the exact
 * message format.
 */
function consolidate<T>(
  trains: TrainClearbox[],
  getter: (cb: ClearBox) => T,
  fieldName: string,
): T {
  const byTrain: Array<[string, T]> = trains.map(({ tid, cb }) => [tid, getter(cb)]);
  const first = byTrain[0][1];
  const allAgree = byTrain.every(([, v]) => v === first);
  if (!allAgree) {
    const detail = byTrain.map(([tid, v]) => `${tid}=${JSON.stringify(v)}`).join(", ");
    throw new Error(`Consolidate conflict on '${fieldName}': trains disagree — ${detail}`);
  }
  return first;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * File_Version 1.1 HDF5 writer. Public MachineConfigWriter dispatches here.
 */
export class Hdf5WriterV1_1 {
  constructor(private readonly config: MachineConfig) {}

  async write(outputPath: string): Promise<void> {
    await ensureReady();
    const path = normPath(outputPath);
    const f = new h5wasm.File(path, "w");
    try {
      const meta = this.config.meta;
      ws(f, "machine_name", meta.machine_name);
      ws(f, "manufacturer", meta.manufacturer);
      ws(f, "model", meta.model);
      ws(f, "serial_number", meta.serial_number);
      ws(f, "File_Version", meta.file_version);
      ws(f, "Export_Date", meta.export_date);
      ws(f, "Configuration_Hash", meta.configuration_hash);
      writeExtra(f, meta.extra);

      // Machine group
      const machineGrp = f.create_group(layout.ROOT_MACHINE);
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
      const trains = this.config.optical_trains;

      // Change 1 (Consolidate): collect every ClearBox-bearing train's
      // Output_Path/Software_Trigger_Delay; hard error on disagreement.
      const trainsWithClearbox: TrainClearbox[] = [];
      for (let i = 0; i < trains.length; i++) {
        const cb = trains[i].optional_components.clearbox;
        if (cb != null) trainsWithClearbox.push({ tid: layout.trainId(i), cb });
      }

      const needsExtensions = trainsWithClearbox.length > 0 || this.config.opcua != null;
      const extensionsGrp = needsExtensions ? f.create_group(layout.ROOT_EXTENSIONS) : null;

      let clearboxRootGrp: h5wasm.Group | null = null;
      if (trainsWithClearbox.length > 0 && extensionsGrp != null) {
        const outputPath = consolidate(trainsWithClearbox, (cb) => cb.output_path, "Output_Path");
        const softwareTriggerDelay = consolidate(
          trainsWithClearbox,
          (cb) => cb.software_trigger_delay,
          "Software_Trigger_Delay",
        );
        clearboxRootGrp = extensionsGrp.create_group("ClearBox");
        ws(clearboxRootGrp, "Output_Path", outputPath);
        wi(clearboxRootGrp, "Software_Trigger_Delay", softwareTriggerDelay);
      }

      for (let i = 0; i < trains.length; i++) {
        const train = trains[i];
        const tid = layout.trainId(i);
        const base = layout.trainPathById(tid);
        const grp = f.create_group(base);
        writeTrainAttrs(grp, train);
        writeScanner(grp.create_group(layout.GROUP_SCANNER), train.scanner);
        writeLightSource(grp.create_group(layout.GROUP_LIGHT_SOURCE), train.light_source);
        writeCollimator(grp.create_group(layout.GROUP_COLLIMATOR), train.collimator);
        writeScannerCard(grp.create_group(layout.GROUP_SCANNER_CARD), train.scanner_card);

        const cb = train.optional_components.clearbox;
        if (cb != null && clearboxRootGrp != null) {
          writeClearBox(clearboxRootGrp.create_group(tid), cb);
        }

        if (train.scan_field_correction_file != null) {
          writeSfcf(grp, train.scan_field_correction_file);
        }
      }

      // OPC-UA (optional) — relocated to Extensions/TM_OPCUA/ (Change 2).
      if (this.config.opcua != null) {
        writeOpcua(f, this.config.opcua);
      }
    } finally {
      f.close();
    }
  }
}
