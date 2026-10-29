/**
 * Mock v1.1 adapter — test artifact only, exercises the change-category
 * architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
 * change. See the mock-migration manifest under `docs/migrations/` for the full detail.
 *
 * Deliberately *not* a `.test.ts` file: `adapterMigration.test.ts` and
 * `scratch/inspectMigration.mts` both import from here. Keeping the mock
 * classes out of a Vitest test file means a plain script can import them
 * without also pulling in `describe`/`it` calls that only make sense inside
 * Vitest's own test runner.
 *
 * Design: fully self-contained — this reader/writer do NOT import or call
 * into the prior version's real adapter module, even for subcomponents (Light_Source,
 * Collimator, Scanner_Card, ClearBox, sfcf, OPCUA) whose on-disk shape
 * happens to be byte-identical to v1.0 today. This mirrors the equivalent
 * fix already applied to `python/tests/test_adapter_migration.py`: if this
 * mock depended on v1.0's real adapter, v1.0 could never be changed or
 * removed later without checking every mock (and, by the same logic, every
 * later real version) that quietly leaned on it. "Unchanged" subcomponents
 * are re-implemented here rather than shared — a large amount of duplication
 * with `hdf5.ts`/`writer.ts` is the intended outcome, not a smell.
 */
import * as h5wasm from 'h5wasm/node';
import { resolve } from 'node:path';

import { MockConfigBuilder } from '../src/builder.js';
import type {
  AxisConfig,
  Collimator,
  LightSource,
  MachineConfig,
  MachineConfigMeta,
  Machine,
  OpticalTrain,
  ScanFieldCorrectionFile,
  Scanner,
  ScannerCard,
} from '../src/models.js';

export interface ReadOptions {
  /** Include the `.fc3` bytes (`raw_bytes`) in the output. */
  includeBinary?: boolean;
}

// ---------------------------------------------------------------------------
// MockV1_1Layout — on-disk constants that differ from v1.0
// ---------------------------------------------------------------------------

export const FILE_VERSION = '1.1-mock';
export const ATTR_FACILITY_ID = 'Facility_ID';
export const ATTR_CONFIG_AUTHOR = 'Config_Author';
export const ATTR_MACHINE_LABEL = 'Machine_Label';
export const ATTR_FOCAL_DISTANCE = 'Focal_Distance';
export const DIMENSIONS_PATH = 'Machine/Dimensions';
export const ATTR_BP_WIDTH = 'Width';
export const ATTR_BP_HEIGHT = 'Height';

const ROOT_MACHINE = 'Machine';
const ROOT_OPTICAL_TRAINS = 'Machine/Optical_Trains';
const TRAIN_ID_PREFIX = 'Optical_Train_';

function trainId(index: number): string {
  return `${TRAIN_ID_PREFIX}${String(index + 1).padStart(2, '0')}`;
}

function trainIds(keys: string[]): string[] {
  return keys.filter((k) => k.startsWith(TRAIN_ID_PREFIX)).sort();
}

function trainPathById(tid: string): string {
  return `${ROOT_OPTICAL_TRAINS}/${tid}`;
}

// ---------------------------------------------------------------------------
// WASM init — lazy singleton, module-local (not shared with v1.0's).
// ---------------------------------------------------------------------------

let _readyPromise: Promise<void> | null = null;

function ensureReady(): Promise<void> {
  if (_readyPromise === null) {
    _readyPromise = h5wasm.ready.then(() => undefined);
  }
  return _readyPromise;
}

function normPath(p: string): string {
  return resolve(p).replace(/\\/g, '/');
}

// ---------------------------------------------------------------------------
// Type aliases and guards
// ---------------------------------------------------------------------------

type H5Attrs = Record<string, h5wasm.Attribute>;

function asGroup(entity: h5wasm.Entity | null, where: string): h5wasm.Group {
  if (entity == null) {
    throw new Error(`HDF5: missing group at "${where}"`);
  }
  if (!(entity instanceof h5wasm.Group)) {
    const kind = 'type' in entity ? (entity as { type: string }).type : entity.constructor.name;
    throw new Error(`HDF5: expected Group at "${where}", got ${kind}`);
  }
  return entity;
}

function asDataset(entity: h5wasm.Entity | null, where: string): h5wasm.Dataset {
  if (entity == null) {
    throw new Error(`HDF5: missing dataset at "${where}"`);
  }
  if (!(entity instanceof h5wasm.Dataset)) {
    const kind = 'type' in entity ? (entity as { type: string }).type : entity.constructor.name;
    throw new Error(`HDF5: expected Dataset at "${where}", got ${kind}`);
  }
  return entity;
}

// ---------------------------------------------------------------------------
// Attribute reading helpers — exact copies of hdf5.ts's helpers
// ---------------------------------------------------------------------------

function attrRaw(attrs: H5Attrs, key: string): string | number | boolean | null {
  const attr = attrs[key];
  if (attr == null) return null;
  const v = attr.value;
  if (v == null) return null;
  if (typeof v === 'bigint') return Number(v);
  if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') return v;
  if (ArrayBuffer.isView(v) && 'length' in v) {
    const arr = v as unknown as ArrayLike<number>;
    return arr.length > 0 ? arr[0] : null;
  }
  if (Array.isArray(v) && v.length > 0) {
    const first = v[0];
    if (typeof first === 'string' || typeof first === 'number' || typeof first === 'boolean') {
      return first;
    }
  }
  return null;
}

function attrStr(attrs: H5Attrs, key: string): string | null {
  const v = attrRaw(attrs, key);
  if (v == null) return null;
  const s = String(v).trim();
  return s || null;
}

function attrStrReq(attrs: H5Attrs, key: string): string {
  return attrStr(attrs, key) ?? '';
}

function attrFloat(attrs: H5Attrs, key: string): number | null {
  const v = attrRaw(attrs, key);
  if (v == null) return null;
  if (typeof v === 'string' && !v.trim()) return null;
  const n = Number(v);
  if (isNaN(n)) {
    throw new TypeError(`Attribute "${key}": cannot parse ${JSON.stringify(v)} as a number`);
  }
  return n;
}

function attrInt(attrs: H5Attrs, key: string): number | null {
  const f = attrFloat(attrs, key);
  return f == null ? null : Math.trunc(f);
}

function attrBool(attrs: H5Attrs, key: string): boolean | null {
  const n = attrInt(attrs, key);
  return n == null ? null : n !== 0;
}

function attrExtra(attrs: H5Attrs, known: Set<string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [k, attr] of Object.entries(attrs)) {
    if (known.has(k)) continue;
    const v = attr.value;
    result[k] = typeof v === 'bigint' ? Number(v) : v;
  }
  return result;
}

const KNOWN_ROOT = new Set([
  'machine_name', 'manufacturer', 'model', 'serial_number',
  'File_Version', 'Export_Date', 'Configuration_Hash',
  ATTR_FACILITY_ID, ATTR_CONFIG_AUTHOR,
]);
const KNOWN_CLIENT = new Set([
  'Server_URL', 'Auth_Mode', 'Security_Mode', 'Security_Policy',
  'BFS_Max_Depth', 'Publish_Interval', 'Sampling_Interval', 'Session_Timeout',
  'Keep_Alive_Count', 'Lifetime_Count', 'Machine_Profile', 'Queue_Policy',
  'Queue_Size_Data_Change', 'Queue_Size_Events', 'Reconnect_Interval', 'Root_Node',
  'Sync_Loop_Interval_Initial', 'Sync_Loop_Interval_Settled',
]);
const KNOWN_PIPE = new Set([
  'Pipe_Enabled', 'Buffer_Size', 'Configure_Client', 'Inbound_Rate_Limit',
  'Max_Inbound_Message_Size', 'Min_Integrity_Level', 'Pipe_Name', 'User_Access_Level',
]);
const KNOWN_TRIGGER = new Set([
  'ID', 'Signal', 'Subsystem', 'Rule_Enabled', 'Start_Value', 'Stop_Value',
  'Case_Sensitivity', 'Component', 'Cooldown_Period', 'Event',
  'Max_Fires_Per_Job', 'Trigger_Label',
]);

// ---------------------------------------------------------------------------
// Parsing functions — one per HDF5 sub-tree, mirroring hdf5.ts field-for-field
// except at the 10 documented mock-v1.1 deltas (marked below).
// ---------------------------------------------------------------------------

function parseMeta(f: h5wasm.File): MachineConfigMeta {
  const a = f.attrs;
  return {
    schema_version: 'v1',
    machine_name: attrStrReq(a, 'machine_name'),
    manufacturer: attrStrReq(a, 'manufacturer'),
    model: attrStrReq(a, 'model'),
    serial_number: attrStrReq(a, 'serial_number'),
    file_version: attrStrReq(a, 'File_Version'),
    export_date: attrStrReq(a, 'Export_Date'),
    configuration_hash: attrStrReq(a, 'Configuration_Hash'),
    // ADDITION (x2): typed fields read from dedicated root attrs.
    facility_id: attrStr(a, ATTR_FACILITY_ID),
    config_author: attrStr(a, ATTR_CONFIG_AUTHOR),
    extra: attrExtra(a, KNOWN_ROOT),
  };
}

function parseMachine(f: h5wasm.File): Machine {
  const grp = asGroup(f.get(ROOT_MACHINE), ROOT_MACHINE);
  const a = grp.attrs;
  const dimsEnt = f.get(DIMENSIONS_PATH);
  const dims = dimsEnt instanceof h5wasm.Group ? dimsEnt.attrs : ({} as H5Attrs);
  return {
    id: attrStr(a, 'ID'),
    // NAME (x1): Machine_Name -> Machine_Label.
    machine_name: attrStrReq(a, ATTR_MACHINE_LABEL),
    manufacturer: attrStrReq(a, 'Manufacturer'),
    model: attrStrReq(a, 'Model'),
    serial_number: attrStrReq(a, 'Serial_Number'),
    // NAME+PATH (x2): X/Y move to Dimensions/ as Width/Height; units stay on Machine/.
    build_plate_x: attrFloat(dims, ATTR_BP_WIDTH),
    build_plate_x_unit: attrStr(a, 'Build_Plate_X_Dimension_unit'),
    build_plate_y: attrFloat(dims, ATTR_BP_HEIGHT),
    build_plate_y_unit: attrStr(a, 'Build_Plate_Y_Dimension_unit'),
    // PATH (x2): Z and radius move to Dimensions/ under the same key.
    build_plate_z: attrFloat(dims, 'Build_Plate_Z_Dimension'),
    build_plate_z_unit: attrStr(a, 'Build_Plate_Z_Dimension_unit'),
    build_plate_radius: attrFloat(dims, 'Build_Plate_Corner_Radius'),
    build_plate_radius_unit: attrStr(a, 'Build_Plate_Corner_Radius_unit'),
    // REMOVAL (x2): no longer present on-disk in mock v1.1 — always null.
    gas_flow_direction: null,
    recoat_direction: null,
    // Not part of this mock's manifest; always null here, same as any other
    // untouched Machine field.
    recoater_blade_type: null,
  };
}

function parseAxis(grp: h5wasm.Group): AxisConfig {
  const a = grp.attrs;
  return {
    actual_bit_resolution: attrInt(a, 'Actual_Bit_Resolution'),
    actual_bit_resolution_unit: attrStr(a, 'Actual_Bit_Resolution_unit'),
    commanded_bit_resolution: attrInt(a, 'Commanded_Bit_Resolution'),
    commanded_bit_resolution_unit: attrStr(a, 'Commanded_Bit_Resolution_unit'),
    control_type: attrStr(a, 'Control_Type'),
    range_of_motion: attrFloat(a, 'Range_Of_Motion'),
    range_of_motion_unit: attrStr(a, 'Range_Of_Motion_unit'),
    smoothing_kernel: attrStr(a, 'Smoothing_Kernel'),
    smoothing_parameters: attrFloat(a, 'Smoothing_Parameters'),
    tuning_parameters: attrStr(a, 'Tuning_Parameters'),
    tuning_type: attrStr(a, 'Tuning_Type'),
  };
}

function parseScanner(grp: h5wasm.Group): Scanner {
  const a = grp.attrs;
  const xEnt = grp.get('X_Axis');
  const yEnt = grp.get('Y_Axis');
  const zEnt = grp.get('Z_Axis');
  const focusEnt = grp.get('Focus');
  return {
    manufacturer: attrStrReq(a, 'Manufacturer'),
    model: attrStrReq(a, 'Model'),
    serial_number: attrStrReq(a, 'Serial_Number'),
    // NAME (x1): Working_Distance -> Focal_Distance.
    working_distance: attrFloat(a, ATTR_FOCAL_DISTANCE),
    working_distance_unit: attrStr(a, 'Working_Distance_unit'),
    scan_field_x: attrFloat(a, 'Scan_Field_Size_X'),
    scan_field_x_unit: attrStr(a, 'Scan_Field_Size_X_unit'),
    scan_field_y: attrFloat(a, 'Scan_Field_Size_Y'),
    scan_field_y_unit: attrStr(a, 'Scan_Field_Size_Y_unit'),
    scan_field_z: attrFloat(a, 'Scan_Field_Size_Z'),
    scan_field_z_unit: attrStr(a, 'Scan_Field_Size_Z_unit'),
    scan_head_offset_x: attrFloat(a, 'Scan_Head_Offset_X'),
    scan_head_offset_x_unit: attrStr(a, 'Scan_Head_Offset_X_unit'),
    scan_head_offset_y: attrFloat(a, 'Scan_Head_Offset_Y'),
    scan_head_offset_y_unit: attrStr(a, 'Scan_Head_Offset_Y_unit'),
    scan_head_offset_z: attrFloat(a, 'Scan_Head_Offset_Z'),
    scan_head_offset_z_unit: attrStr(a, 'Scan_Head_Offset_Z_unit'),
    scan_head_rotation: attrFloat(a, 'Scan_Head_Rotation'),
    scan_head_rotation_unit: attrStr(a, 'Scan_Head_Rotation_unit'),
    axis_configuration: attrStr(a, 'Axis_Configuration'),
    x_axis: xEnt ? parseAxis(asGroup(xEnt, 'Scanner/X_Axis')) : null,
    y_axis: yEnt ? parseAxis(asGroup(yEnt, 'Scanner/Y_Axis')) : null,
    z_axis: zEnt ? parseAxis(asGroup(zEnt, 'Scanner/Z_Axis')) : null,
    focus: focusEnt ? parseAxis(asGroup(focusEnt, 'Scanner/Focus')) : null,
    invert_actual_x: attrBool(a, 'Invert_Actual_X') ?? false,
    invert_actual_y: attrBool(a, 'Invert_Actual_Y') ?? false,
    invert_commanded_x: attrBool(a, 'Invert_Commanded_X') ?? false,
    invert_commanded_y: attrBool(a, 'Invert_Commanded_Y') ?? false,
  };
}

function parseLightSource(grp: h5wasm.Group): LightSource {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, 'Manufacturer'),
    model: attrStrReq(a, 'Model'),
    serial_number: attrStrReq(a, 'Serial_Number'),
    wavelength: attrFloat(a, 'Light_Wavelength'),
    wavelength_unit: attrStr(a, 'Light_Wavelength_unit'),
    power_max_nominal: attrFloat(a, 'Power_Max_Nominal'),
    power_max_nominal_unit: attrStr(a, 'Power_Max_Nominal_unit'),
    power_max_actual: attrFloat(a, 'Power_Max_Actual'),
    power_max_actual_unit: attrStr(a, 'Power_Max_Actual_unit'),
    power_min_actual: attrFloat(a, 'Power_Min_Actual'),
    power_min_actual_unit: attrStr(a, 'Power_Min_Actual_unit'),
    power_min_nominal: attrFloat(a, 'Power_Min_Nominal'),
    power_min_nominal_unit: attrStr(a, 'Power_Min_Nominal_unit'),
    power_bit_resolution: attrFloat(a, 'Power_Bit_Resolution'),
    power_bit_resolution_unit: attrStr(a, 'Power_Bit_Resolution_unit'),
  };
}

function parseCollimator(grp: h5wasm.Group): Collimator {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, 'Manufacturer'),
    model: attrStrReq(a, 'Model'),
    serial_number: attrStrReq(a, 'Serial_Number'),
    focal_length: attrFloat(a, 'Focal_Length'),
    focal_length_unit: attrStr(a, 'Focal_Length_unit'),
  };
}

function parseScannerCard(grp: h5wasm.Group): ScannerCard {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, 'Manufacturer'),
    model: attrStrReq(a, 'Model'),
    serial_number: attrStrReq(a, 'Serial_Number'),
    communication_protocol: attrStr(a, 'Communication_Protocol'),
    sample_period: attrFloat(a, 'Sample_Period'),
    sample_period_unit: attrStr(a, 'Sample_Period_unit'),
  };
}

function parseSfcf(ds: h5wasm.Dataset, includeBinary: boolean): ScanFieldCorrectionFile {
  const a = ds.attrs;
  const sfcf: ScanFieldCorrectionFile = {
    document_name: attrStrReq(a, 'document_name'),
    document_id: attrStrReq(a, 'document_id'),
    file_size: attrInt(a, 'file_size') ?? 0,
    valid_as_of_date: attrStrReq(a, 'valid_as_of_date'),
    document_created_at: attrStr(a, 'document_created_at'),
    document_type: attrStr(a, 'document_type'),
    original_uri: attrStr(a, 'original_uri'),
  };

  if (includeBinary) {
    const raw = ds.value;
    sfcf.raw_bytes = raw instanceof Uint8Array ? Buffer.from(raw).toString('base64') : null;
  }

  return sfcf;
}

function parseOpticalTrain(f: h5wasm.File, tid: string, includeBinary: boolean): OpticalTrain {
  const base = trainPathById(tid);
  const trainGrp = asGroup(f.get(base), base);
  const a = trainGrp.attrs;

  const sfcfEnt = trainGrp.get('scan_field_correction_file');

  const sfcf: ScanFieldCorrectionFile | null =
    sfcfEnt instanceof h5wasm.Dataset ? parseSfcf(sfcfEnt, includeBinary) : null;

  return {
    train_id: tid,
    id: attrStr(a, 'ID'),
    beam_profile_type: attrStr(a, 'Beam_Profile_Type'),
    beam_waist_definition: attrStr(a, 'Beam_Waist_Definition'),
    beam_waist_major: attrFloat(a, 'Beam_Waist_Major'),
    beam_waist_major_unit: attrStr(a, 'Beam_Waist_Major_unit'),
    beam_waist_minor: attrFloat(a, 'Beam_Waist_Minor'),
    beam_waist_minor_unit: attrStr(a, 'Beam_Waist_Minor_unit'),
    beam_waist_offset_z: attrFloat(a, 'Beam_Waist_Offset_Z'),
    beam_waist_offset_z_unit: attrStr(a, 'Beam_Waist_Offset_Z_unit'),
    build_plane_offset_major: attrFloat(a, 'Build_Plane_Offset_Major'),
    build_plane_offset_major_unit: attrStr(a, 'Build_Plane_Offset_Major_unit'),
    build_plane_offset_minor: attrFloat(a, 'Build_Plane_Offset_Minor'),
    build_plane_offset_minor_unit: attrStr(a, 'Build_Plane_Offset_Minor_unit'),
    collimator_focal_length: attrFloat(a, 'Collimator_Focal_Length'),
    collimator_focal_length_unit: attrStr(a, 'Collimator_Focal_Length_unit'),
    m2_major: attrFloat(a, 'M2_Major'),
    m2_minor: attrFloat(a, 'M2_Minor'),
    major_axis_angle: attrFloat(a, 'Major_Axis_Angle'),
    major_axis_angle_unit: attrStr(a, 'Major_Axis_Angle_unit'),
    rayleigh_length_major: attrFloat(a, 'Rayleigh_Length_Major'),
    rayleigh_length_major_unit: attrStr(a, 'Rayleigh_Length_Major_unit'),
    rayleigh_length_minor: attrFloat(a, 'Rayleigh_Length_Minor'),
    rayleigh_length_minor_unit: attrStr(a, 'Rayleigh_Length_Minor_unit'),
    scanner_number: attrStr(a, 'Scanner_Number'),
    thermal_lensing_passed: attrBool(a, 'Thermal_Lensing_Test_Passed'),
    thermal_lensing_focal_plane_shift: attrFloat(a, 'Thermal_Lensing_Focal_Plane_Shift'),
    thermal_lensing_focal_plane_shift_unit: attrStr(a, 'Thermal_Lensing_Focal_Plane_Shift_unit'),
    thermal_lensing_threshold: attrFloat(a, 'Thermal_Lensing_Threshold'),
    thermal_lensing_threshold_unit: attrStr(a, 'Thermal_Lensing_Threshold_unit'),
    scanner: parseScanner(asGroup(trainGrp.get('Scanner'), `${base}/Scanner`)),
    light_source: parseLightSource(asGroup(trainGrp.get('Light_Source'), `${base}/Light_Source`)),
    collimator: parseCollimator(asGroup(trainGrp.get('Collimator'), `${base}/Collimator`)),
    scanner_card: parseScannerCard(asGroup(trainGrp.get('Scanner_Card'), `${base}/Scanner_Card`)),
    scan_field_correction_file: sfcf,
  };
}

function parseFile(f: h5wasm.File, opts: ReadOptions): MachineConfig {
  const includeBinary = opts.includeBinary ?? false;

  const meta = parseMeta(f);
  const machine = parseMachine(f);

  const trainsGrp = asGroup(f.get(ROOT_OPTICAL_TRAINS), ROOT_OPTICAL_TRAINS);
  const tids = trainIds(trainsGrp.keys());

  const optical_trains = tids.map((tid) => parseOpticalTrain(f, tid, includeBinary));

  const config: MachineConfig = { meta, machine, optical_trains };
  return config;
}

// ---------------------------------------------------------------------------
// MockV1_1Reader — fully self-contained parser (no v1.0 delegation)
// ---------------------------------------------------------------------------

export class MockV1_1Reader {
  private readonly _path: string;

  constructor(path: string) {
    this._path = normPath(path);
  }

  async parse(options: ReadOptions = {}): Promise<MachineConfig> {
    await ensureReady();
    const f = new h5wasm.File(this._path, 'r');
    try {
      return parseFile(f, options);
    } finally {
      f.close();
    }
  }
}

// ---------------------------------------------------------------------------
// Write helpers — exact copies of writer.ts's helpers
// ---------------------------------------------------------------------------

function ws(grp: h5wasm.Group, key: string, val: string | null | undefined): void {
  grp.create_attribute(key, val ?? '', [], 'S');
}

function wf(grp: h5wasm.Group, key: string, val: number | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, '', [], 'S');
  } else {
    grp.create_attribute(key, val, [], '<d');
  }
}

function wi(grp: h5wasm.Group, key: string, val: number | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, '', [], 'S');
  } else {
    grp.create_attribute(key, Math.trunc(val), [], '<i8');
  }
}

function wb(grp: h5wasm.Group, key: string, val: boolean | null | undefined): void {
  if (val == null) {
    grp.create_attribute(key, '', [], 'S');
  } else {
    grp.create_attribute(key, val ? 1 : 0, [], '<i8');
  }
}

function wbIfTrue(grp: h5wasm.Group, key: string, val: boolean): void {
  if (val) {
    grp.create_attribute(key, 1, [], '<i8');
  }
}

function ws_ds(ds: h5wasm.Dataset, key: string, val: string): void {
  ds.create_attribute(key, val, [], 'S');
}

function wi_ds(ds: h5wasm.Dataset, key: string, val: number): void {
  ds.create_attribute(key, Math.trunc(val), [], '<i8');
}

function writeExtra(grp: h5wasm.Group, extra: Record<string, unknown>): void {
  for (const [key, val] of Object.entries(extra)) {
    if (typeof val === 'string') {
      grp.create_attribute(key, val, [], 'S');
    } else if (typeof val === 'number') {
      if (Number.isInteger(val)) {
        grp.create_attribute(key, val, [], '<i8');
      } else {
        grp.create_attribute(key, val, [], '<d');
      }
    } else if (typeof val === 'boolean') {
      grp.create_attribute(key, val ? 1 : 0, [], '<i8');
    } else if (val != null) {
      grp.create_attribute(key, JSON.stringify(val), [], 'S');
    }
  }
}

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
  wi(grp, 'Actual_Bit_Resolution', a.actual_bit_resolution);
  ws(grp, 'Actual_Bit_Resolution_unit', a.actual_bit_resolution_unit);
  wi(grp, 'Commanded_Bit_Resolution', a.commanded_bit_resolution);
  ws(grp, 'Commanded_Bit_Resolution_unit', a.commanded_bit_resolution_unit);
  ws(grp, 'Control_Type', a.control_type);
  wf(grp, 'Range_Of_Motion', a.range_of_motion);
  ws(grp, 'Range_Of_Motion_unit', a.range_of_motion_unit);
  ws(grp, 'Smoothing_Kernel', a.smoothing_kernel);
  wf(grp, 'Smoothing_Parameters', a.smoothing_parameters);
  ws(grp, 'Tuning_Parameters', a.tuning_parameters);
  ws(grp, 'Tuning_Type', a.tuning_type);
}

function writeScanner(grp: h5wasm.Group, s: Scanner): void {
  ws(grp, 'Manufacturer', s.manufacturer);
  ws(grp, 'Model', s.model);
  ws(grp, 'Serial_Number', s.serial_number);
  // NAME (x1): Working_Distance -> Focal_Distance.
  wf(grp, ATTR_FOCAL_DISTANCE, s.working_distance);
  ws(grp, 'Working_Distance_unit', s.working_distance_unit ?? 'mm');
  wf(grp, 'Scan_Field_Size_X', s.scan_field_x);
  ws(grp, 'Scan_Field_Size_X_unit', s.scan_field_x_unit ?? 'mm');
  wf(grp, 'Scan_Field_Size_Y', s.scan_field_y);
  ws(grp, 'Scan_Field_Size_Y_unit', s.scan_field_y_unit ?? 'mm');
  wf(grp, 'Scan_Field_Size_Z', s.scan_field_z);
  ws(grp, 'Scan_Field_Size_Z_unit', s.scan_field_z_unit ?? 'mm');
  wf(grp, 'Scan_Head_Offset_X', s.scan_head_offset_x);
  ws(grp, 'Scan_Head_Offset_X_unit', s.scan_head_offset_x_unit ?? 'mm');
  wf(grp, 'Scan_Head_Offset_Y', s.scan_head_offset_y);
  ws(grp, 'Scan_Head_Offset_Y_unit', s.scan_head_offset_y_unit ?? 'mm');
  wf(grp, 'Scan_Head_Offset_Z', s.scan_head_offset_z);
  ws(grp, 'Scan_Head_Offset_Z_unit', s.scan_head_offset_z_unit ?? 'mm');
  wf(grp, 'Scan_Head_Rotation', s.scan_head_rotation);
  ws(grp, 'Scan_Head_Rotation_unit', s.scan_head_rotation_unit ?? 'degrees');
  ws(grp, 'Axis_Configuration', s.axis_configuration);
  wbIfTrue(grp, 'Invert_Actual_X', s.invert_actual_x);
  wbIfTrue(grp, 'Invert_Actual_Y', s.invert_actual_y);
  wbIfTrue(grp, 'Invert_Commanded_X', s.invert_commanded_x);
  wbIfTrue(grp, 'Invert_Commanded_Y', s.invert_commanded_y);
  writeAxis(grp.create_group('X_Axis'), s.x_axis);
  writeAxis(grp.create_group('Y_Axis'), s.y_axis);
  if (s.z_axis != null) writeAxis(grp.create_group('Z_Axis'), s.z_axis);
  if (s.focus != null) writeAxis(grp.create_group('Focus'), s.focus);
}

function writeLightSource(grp: h5wasm.Group, ls: LightSource): void {
  ws(grp, 'Manufacturer', ls.manufacturer);
  ws(grp, 'Model', ls.model);
  ws(grp, 'Serial_Number', ls.serial_number);
  wf(grp, 'Light_Wavelength', ls.wavelength);
  ws(grp, 'Light_Wavelength_unit', ls.wavelength_unit ?? 'nm');
  wf(grp, 'Power_Max_Nominal', ls.power_max_nominal);
  ws(grp, 'Power_Max_Nominal_unit', ls.power_max_nominal_unit ?? 'W');
  wf(grp, 'Power_Max_Actual', ls.power_max_actual);
  ws(grp, 'Power_Max_Actual_unit', ls.power_max_actual_unit ?? 'W');
  wf(grp, 'Power_Min_Actual', ls.power_min_actual);
  ws(grp, 'Power_Min_Actual_unit', ls.power_min_actual_unit ?? 'W');
  wf(grp, 'Power_Min_Nominal', ls.power_min_nominal);
  ws(grp, 'Power_Min_Nominal_unit', ls.power_min_nominal_unit ?? 'W');
  ws(grp, 'Power_Bit_Resolution', ls.power_bit_resolution != null ? String(ls.power_bit_resolution) : null);
  ws(grp, 'Power_Bit_Resolution_unit', ls.power_bit_resolution_unit ?? 'bits');
}

function writeCollimator(grp: h5wasm.Group, c: Collimator): void {
  ws(grp, 'Manufacturer', c.manufacturer);
  ws(grp, 'Model', c.model);
  ws(grp, 'Serial_Number', c.serial_number);
  wf(grp, 'Focal_Length', c.focal_length);
  ws(grp, 'Focal_Length_unit', c.focal_length_unit ?? 'mm');
}

function writeScannerCard(grp: h5wasm.Group, sc: ScannerCard): void {
  ws(grp, 'Manufacturer', sc.manufacturer);
  ws(grp, 'Model', sc.model);
  ws(grp, 'Serial_Number', sc.serial_number);
  ws(grp, 'Communication_Protocol', sc.communication_protocol);
  wf(grp, 'Sample_Period', sc.sample_period);
  ws(grp, 'Sample_Period_unit', sc.sample_period_unit ?? 'μs');
}

function writeSfcf(trainGrp: h5wasm.Group, sfcf: ScanFieldCorrectionFile): void {
  let bytes: Uint8Array;
  if (sfcf.raw_bytes) {
    bytes = Buffer.from(sfcf.raw_bytes, 'base64');
  } else {
    const len = Math.max(sfcf.file_size ?? 1, 1);
    bytes = new Uint8Array(len);
  }
  const ds = trainGrp.create_dataset({
    name: 'scan_field_correction_file',
    data: bytes,
    shape: [bytes.length],
    dtype: '<B',
  });
  ws_ds(ds, 'document_name', sfcf.document_name);
  ws_ds(ds, 'document_id', sfcf.document_id);
  wi_ds(ds, 'file_size', sfcf.file_size);
  ws_ds(ds, 'valid_as_of_date', sfcf.valid_as_of_date);
  ws_ds(ds, 'document_created_at', sfcf.document_created_at ?? '');
  ws_ds(ds, 'document_type', sfcf.document_type ?? '');
  ws_ds(ds, 'original_uri', sfcf.original_uri ?? '');
}

function writeOpticalTrain(trainsGrp: h5wasm.Group, trainIdx: number, train: OpticalTrain): void {
  const tid = trainId(trainIdx);
  const trainGrp = trainsGrp.create_group(tid);

  ws(trainGrp, 'ID', train.id);
  ws(trainGrp, 'Beam_Profile_Type', train.beam_profile_type);
  ws(trainGrp, 'Beam_Waist_Definition', train.beam_waist_definition);
  wf(trainGrp, 'Beam_Waist_Major', train.beam_waist_major);
  ws(trainGrp, 'Beam_Waist_Major_unit', train.beam_waist_major_unit ?? 'μm');
  wf(trainGrp, 'Beam_Waist_Minor', train.beam_waist_minor);
  ws(trainGrp, 'Beam_Waist_Minor_unit', train.beam_waist_minor_unit ?? 'μm');
  wf(trainGrp, 'Beam_Waist_Offset_Z', train.beam_waist_offset_z);
  ws(trainGrp, 'Beam_Waist_Offset_Z_unit', train.beam_waist_offset_z_unit ?? 'mm');
  wf(trainGrp, 'Build_Plane_Offset_Major', train.build_plane_offset_major);
  ws(trainGrp, 'Build_Plane_Offset_Major_unit', train.build_plane_offset_major_unit ?? 'mm');
  wf(trainGrp, 'Build_Plane_Offset_Minor', train.build_plane_offset_minor);
  ws(trainGrp, 'Build_Plane_Offset_Minor_unit', train.build_plane_offset_minor_unit ?? 'mm');
  wf(trainGrp, 'Collimator_Focal_Length', train.collimator_focal_length);
  ws(trainGrp, 'Collimator_Focal_Length_unit', train.collimator_focal_length_unit ?? 'mm');
  wf(trainGrp, 'M2_Major', train.m2_major);
  wf(trainGrp, 'M2_Minor', train.m2_minor);
  wf(trainGrp, 'Major_Axis_Angle', train.major_axis_angle);
  ws(trainGrp, 'Major_Axis_Angle_unit', train.major_axis_angle_unit ?? 'degrees');
  wf(trainGrp, 'Rayleigh_Length_Major', train.rayleigh_length_major);
  ws(trainGrp, 'Rayleigh_Length_Major_unit', train.rayleigh_length_major_unit ?? 'mm');
  wf(trainGrp, 'Rayleigh_Length_Minor', train.rayleigh_length_minor);
  ws(trainGrp, 'Rayleigh_Length_Minor_unit', train.rayleigh_length_minor_unit ?? 'mm');
  ws(trainGrp, 'Scanner_Number', train.scanner_number);
  wb(trainGrp, 'Thermal_Lensing_Test_Passed', train.thermal_lensing_passed);
  wf(trainGrp, 'Thermal_Lensing_Focal_Plane_Shift', train.thermal_lensing_focal_plane_shift);
  ws(trainGrp, 'Thermal_Lensing_Focal_Plane_Shift_unit', train.thermal_lensing_focal_plane_shift_unit ?? 'mm');
  wf(trainGrp, 'Thermal_Lensing_Threshold', train.thermal_lensing_threshold);
  ws(trainGrp, 'Thermal_Lensing_Threshold_unit', train.thermal_lensing_threshold_unit ?? 'mm');

  writeScanner(trainGrp.create_group('Scanner'), train.scanner);
  writeLightSource(trainGrp.create_group('Light_Source'), train.light_source);
  writeCollimator(trainGrp.create_group('Collimator'), train.collimator);
  writeScannerCard(trainGrp.create_group('Scanner_Card'), train.scanner_card);

  if (train.scan_field_correction_file != null) {
    writeSfcf(trainGrp, train.scan_field_correction_file);
  }
}

// ---------------------------------------------------------------------------
// MockV1_1Writer — fully self-contained writer (no v1.0 delegation)
// ---------------------------------------------------------------------------

export class MockV1_1Writer {
  constructor(private readonly config: MachineConfig) {}

  async write(outputPath: string): Promise<void> {
    await ensureReady();
    const path = normPath(outputPath);
    const f = new h5wasm.File(path, 'w');
    try {
      const meta = this.config.meta;
      ws(f, 'machine_name', meta.machine_name);
      ws(f, 'manufacturer', meta.manufacturer);
      ws(f, 'model', meta.model);
      ws(f, 'serial_number', meta.serial_number);
      ws(f, 'File_Version', meta.file_version);
      ws(f, 'Export_Date', meta.export_date);
      ws(f, 'Configuration_Hash', meta.configuration_hash);
      // ADDITION (x2): always written, even if empty — schema-complete file.
      ws(f, ATTR_FACILITY_ID, meta.facility_id ?? '');
      ws(f, ATTR_CONFIG_AUTHOR, meta.config_author ?? '');
      writeExtra(f, meta.extra);

      const machineGrp = f.create_group('Machine');
      const ma = this.config.machine;
      ws(machineGrp, 'ID', ma.id);
      // NAME (x1): Machine_Name -> Machine_Label.
      ws(machineGrp, ATTR_MACHINE_LABEL, ma.machine_name);
      ws(machineGrp, 'Manufacturer', ma.manufacturer);
      ws(machineGrp, 'Model', ma.model);
      ws(machineGrp, 'Serial_Number', ma.serial_number);
      // REMOVAL (x2): Gas_Flow_Direction / Recoat_Direction intentionally omitted.

      // Unit attrs stay on Machine/ (path unchanged) per the manifest.
      ws(machineGrp, 'Build_Plate_X_Dimension_unit', ma.build_plate_x_unit ?? 'mm');
      ws(machineGrp, 'Build_Plate_Y_Dimension_unit', ma.build_plate_y_unit ?? 'mm');
      ws(machineGrp, 'Build_Plate_Z_Dimension_unit', ma.build_plate_z_unit ?? 'mm');
      ws(machineGrp, 'Build_Plate_Corner_Radius_unit', ma.build_plate_radius_unit ?? 'mm');

      // NAME+PATH (x2) / PATH (x2): dimension values move into Machine/Dimensions/.
      const dimsGrp = machineGrp.create_group('Dimensions');
      wf(dimsGrp, ATTR_BP_WIDTH, ma.build_plate_x);
      wf(dimsGrp, ATTR_BP_HEIGHT, ma.build_plate_y);
      wf(dimsGrp, 'Build_Plate_Z_Dimension', ma.build_plate_z);
      wf(dimsGrp, 'Build_Plate_Corner_Radius', ma.build_plate_radius);

      const trainsGrp = machineGrp.create_group('Optical_Trains');
      for (let i = 0; i < this.config.optical_trains.length; i++) {
        writeOpticalTrain(trainsGrp, i, this.config.optical_trains[i]);
      }
    } finally {
      f.close();
    }
  }
}

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's `_make_config()`
// ---------------------------------------------------------------------------

export function makeMockConfig(opts: {
  machineName?: string;
  facilityId?: string | null;
  configAuthor?: string | null;
} = {}): MachineConfig {
  const cfg = new MockConfigBuilder({
    nLasers: 1,
    buildPlateX: 250.0,
    buildPlateY: 175.0, // distinct from x so name+path assertions are unambiguous
    fileVersion: FILE_VERSION,
    machineName: opts.machineName ?? 'MigrationTestMachine',
  }).build();
  return {
    ...cfg,
    meta: {
      ...cfg.meta,
      facility_id: opts.facilityId ?? null,
      config_author: opts.configAuthor ?? null,
    },
    machine: {
      ...cfg.machine,
      gas_flow_direction: null, // absent in v1.1-mock by design
      recoat_direction: null,
    },
  };
}
