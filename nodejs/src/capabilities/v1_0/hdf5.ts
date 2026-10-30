/**
 * File_Version 1.0 HDF5 reader — on-disk layout, attribute names, and casting.
 * Public MachineConfigReader peeks File_Version then dispatches here.
 */
import * as h5wasm from "h5wasm/node";
import { resolve } from "node:path";
import type {
  MachineConfig,
  MachineConfigMeta,
  Machine,
  OpticalTrain,
  Scanner,
  AxisConfig,
  LightSource,
  Collimator,
  ScannerCard,
  ScanFieldCorrectionFile,
} from "../../models.js";
import { forwardPowerCharacterizationPoints } from "../../powerCharacterization.js";
import * as layout from "./layout.js";
import { readExtensionTree } from "../../extensions/hdf5.js";
import type { ExtensionTree } from "../../extensions/contract.js";
import { withExtensionsAsJson } from "../../extensions/json.js";
import { CLEARBOX_EXTENSION_NAME, clearboxFromV1_0Trains } from "../../clearboxLegacy.js";

export interface ReadOptions {
  /** Include the `.fc3` bytes (`raw_bytes`) in the output. Extension datasets are always complete. */
  includeBinary?: boolean;
}

export interface ToJsonOptions extends ReadOptions {
  /** JSON indentation spaces (default 2). Pass 0 for compact. */
  indent?: number;
}

// ---------------------------------------------------------------------------
// WASM init — lazy singleton
// ---------------------------------------------------------------------------

let _readyPromise: Promise<void> | null = null;

function ensureReady(): Promise<void> {
  if (_readyPromise === null) {
    _readyPromise = h5wasm.ready.then(() => undefined);
  }
  return _readyPromise;
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/** Normalise a host-filesystem path to POSIX-style for h5wasm NODERAWFS. */
function normPath(p: string): string {
  return resolve(p).replace(/\\/g, "/");
}

// ---------------------------------------------------------------------------
// Type aliases and guards
// ---------------------------------------------------------------------------

type H5Attrs = Record<string, h5wasm.Attribute>;

/** Assert that an entity is a Group; throw with a descriptive message if not. */
function asGroup(entity: h5wasm.Entity | null, where: string): h5wasm.Group {
  if (entity == null) {
    throw new Error(`HDF5: missing group at "${where}"`);
  }
  if (!(entity instanceof h5wasm.Group)) {
    const kind = "type" in entity ? (entity as { type: string }).type : entity.constructor.name;
    throw new Error(`HDF5: expected Group at "${where}", got ${kind}`);
  }
  return entity;
}

/** Assert that an entity is a Dataset; throw with a descriptive message if not. */
function asDataset(entity: h5wasm.Entity | null, where: string): h5wasm.Dataset {
  if (entity == null) {
    throw new Error(`HDF5: missing dataset at "${where}"`);
  }
  if (!(entity instanceof h5wasm.Dataset)) {
    const kind = "type" in entity ? (entity as { type: string }).type : entity.constructor.name;
    throw new Error(`HDF5: expected Dataset at "${where}", got ${kind}`);
  }
  return entity;
}

// ---------------------------------------------------------------------------
// Attribute reading helpers
// ---------------------------------------------------------------------------

/**
 * Extract a primitive scalar from an h5wasm Attribute value.
 * Returns null for missing attributes, empty strings, or non-scalar types.
 */
function attrRaw(attrs: H5Attrs, key: string): string | number | boolean | null {
  const attr = attrs[key];
  if (attr == null) return null;
  const v = attr.value;
  if (v == null) return null;
  if (typeof v === "bigint") return Number(v);
  if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") return v;
  // 1-element TypedArray (some HDF5 scalars are stored as length-1 arrays)
  if (ArrayBuffer.isView(v) && "length" in v) {
    const arr = v as unknown as ArrayLike<number>;
    return arr.length > 0 ? arr[0] : null;
  }
  if (Array.isArray(v) && v.length > 0) {
    const first = v[0];
    if (typeof first === "string" || typeof first === "number" || typeof first === "boolean") {
      return first;
    }
  }
  return null;
}

/** Read a string attribute; null if absent, whitespace-only, or non-string. */
function attrStr(attrs: H5Attrs, key: string): string | null {
  const v = attrRaw(attrs, key);
  if (v == null) return null;
  const s = String(v).trim();
  return s || null;
}

/** Like attrStr but falls back to "" instead of null. */
function attrStrReq(attrs: H5Attrs, key: string): string {
  return attrStr(attrs, key) ?? "";
}

/**
 * Read a float attribute; null if absent or empty.
 * Throws if the attribute is present but cannot be parsed as a number —
 * a corrupt/non-numeric value must fail loudly, not silently become null.
 */
function attrFloat(attrs: H5Attrs, key: string): number | null {
  const v = attrRaw(attrs, key);
  if (v == null) return null;
  if (typeof v === "string" && !v.trim()) return null;
  const n = Number(v);
  if (isNaN(n)) {
    throw new TypeError(`Attribute "${key}": cannot parse ${JSON.stringify(v)} as a number`);
  }
  return n;
}

/** Read an integer attribute (truncates towards zero). */
function attrInt(attrs: H5Attrs, key: string): number | null {
  const f = attrFloat(attrs, key);
  return f == null ? null : Math.trunc(f);
}

/** Read a boolean stored as 0/1 integer. */
function attrBool(attrs: H5Attrs, key: string): boolean | null {
  const n = attrInt(attrs, key);
  return n == null ? null : n !== 0;
}

/**
 * Collect all attributes NOT in `known` into a plain record.
 * Used for the `extra` fields on Meta.
 */
function attrExtra(attrs: H5Attrs, known: Set<string>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [k, attr] of Object.entries(attrs)) {
    if (known.has(k)) continue;
    const v = attr.value;
    result[k] = typeof v === "bigint" ? Number(v) : v;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Known HDF5 attribute sets (used to separate "typed" from "extra" fields)
// ---------------------------------------------------------------------------

const KNOWN_ROOT = new Set([
  "machine_name", "manufacturer", "model", "serial_number",
  "File_Version", "Export_Date", "Configuration_Hash",
]);

// ---------------------------------------------------------------------------
// Dataset helpers
// ---------------------------------------------------------------------------

/**
 * `JSON.stringify` replacer: drops Scanner's four `invert_*` keys when their
 * value is exactly `false`. Only `true` ever appears in JSON output — the
 * model/API layer keeps a plain, always-true-or-false field (never
 * `undefined`); this is the one place the "omit unless true" business rule
 * (user-confirmed, 2026-08-21) actually applies, since there's no separate
 * to-dict serialisation layer in this language to hold it instead. Also
 * unconditionally drops `is_valid` (`MachineConfigMeta`) — reader-computed,
 * in-memory only, never part of the interop contract; the same "no
 * to-dict layer" reasoning applies, so it must be actively stripped here.
 */
const INVERT_FLAG_KEYS = new Set([
  "invert_actual_x", "invert_actual_y", "invert_commanded_x", "invert_commanded_y",
]);
function omitFalseInvertFlags(key: string, value: unknown): unknown {
  if (key === "is_valid") return undefined;
  if (INVERT_FLAG_KEYS.has(key) && value === false) return undefined;
  return value;
}

// ---------------------------------------------------------------------------
// Parsing functions — one per HDF5 sub-tree
// ---------------------------------------------------------------------------

function parseMeta(f: h5wasm.File): MachineConfigMeta {
  const a = f.attrs;
  return {
    schema_version: "v1",
    machine_name: attrStrReq(a, "machine_name"),
    manufacturer: attrStrReq(a, "manufacturer"),
    model: attrStrReq(a, "model"),
    serial_number: attrStrReq(a, "serial_number"),
    file_version: attrStrReq(a, "File_Version"),
    export_date: attrStrReq(a, "Export_Date"),
    configuration_hash: attrStrReq(a, "Configuration_Hash"),
    extra: attrExtra(a, KNOWN_ROOT),
  };
}

function parseMachine(f: h5wasm.File): Machine {
  const grp = asGroup(f.get(layout.ROOT_MACHINE), layout.ROOT_MACHINE);
  const a = grp.attrs;
  return {
    id: attrStr(a, "ID"),
    machine_name: attrStrReq(a, "Machine_Name"),
    manufacturer: attrStrReq(a, "Manufacturer"),
    model: attrStrReq(a, "Model"),
    serial_number: attrStrReq(a, "Serial_Number"),
    build_plate_x: attrFloat(a, "Build_Plate_X_Dimension"),
    build_plate_x_unit: attrStr(a, "Build_Plate_X_Dimension_unit"),
    build_plate_y: attrFloat(a, "Build_Plate_Y_Dimension"),
    build_plate_y_unit: attrStr(a, "Build_Plate_Y_Dimension_unit"),
    build_plate_z: attrFloat(a, "Build_Plate_Z_Dimension"),
    build_plate_z_unit: attrStr(a, "Build_Plate_Z_Dimension_unit"),
    build_plate_radius: attrFloat(a, "Build_Plate_Corner_Radius"),
    build_plate_radius_unit: attrStr(a, "Build_Plate_Corner_Radius_unit"),
    gas_flow_direction: attrStr(a, "Gas_Flow_Direction"),
    recoat_direction: attrStr(a, "Recoat_Direction"),
    recoater_blade_type: attrStr(a, "Recoater_Blade_Type"),
  };
}

function parseAxis(grp: h5wasm.Group): AxisConfig {
  const a = grp.attrs;
  return {
    actual_bit_resolution: attrInt(a, "Actual_Bit_Resolution"),
    actual_bit_resolution_unit: attrStr(a, "Actual_Bit_Resolution_unit"),
    commanded_bit_resolution: attrInt(a, "Commanded_Bit_Resolution"),
    commanded_bit_resolution_unit: attrStr(a, "Commanded_Bit_Resolution_unit"),
    control_type: attrStr(a, "Control_Type"),
    range_of_motion: attrFloat(a, "Range_Of_Motion"),
    range_of_motion_unit: attrStr(a, "Range_Of_Motion_unit"),
    smoothing_kernel: attrStr(a, "Smoothing_Kernel"),
    smoothing_parameters: attrFloat(a, "Smoothing_Parameters"),
    tuning_parameters: attrStr(a, "Tuning_Parameters"),
    tuning_type: attrStr(a, "Tuning_Type"),
  };
}

function parseScanner(grp: h5wasm.Group): Scanner {
  const a = grp.attrs;
  const xEnt = grp.get("X_Axis");
  const yEnt = grp.get("Y_Axis");
  const zEnt = grp.get("Z_Axis");
  const focusEnt = grp.get("Focus");
  return {
    manufacturer: attrStrReq(a, "Manufacturer"),
    model: attrStrReq(a, "Model"),
    serial_number: attrStrReq(a, "Serial_Number"),
    working_distance: attrFloat(a, "Working_Distance"),
    working_distance_unit: attrStr(a, "Working_Distance_unit"),
    scan_field_x: attrFloat(a, "Scan_Field_Size_X"),
    scan_field_x_unit: attrStr(a, "Scan_Field_Size_X_unit"),
    scan_field_y: attrFloat(a, "Scan_Field_Size_Y"),
    scan_field_y_unit: attrStr(a, "Scan_Field_Size_Y_unit"),
    scan_field_z: attrFloat(a, "Scan_Field_Size_Z"),
    scan_field_z_unit: attrStr(a, "Scan_Field_Size_Z_unit"),
    scan_head_offset_x: attrFloat(a, "Scan_Head_Offset_X"),
    scan_head_offset_x_unit: attrStr(a, "Scan_Head_Offset_X_unit"),
    scan_head_offset_y: attrFloat(a, "Scan_Head_Offset_Y"),
    scan_head_offset_y_unit: attrStr(a, "Scan_Head_Offset_Y_unit"),
    scan_head_offset_z: attrFloat(a, "Scan_Head_Offset_Z"),
    scan_head_offset_z_unit: attrStr(a, "Scan_Head_Offset_Z_unit"),
    scan_head_rotation: attrFloat(a, "Scan_Head_Rotation"),
    scan_head_rotation_unit: attrStr(a, "Scan_Head_Rotation_unit"),
    axis_configuration: attrStr(a, "Axis_Configuration"),
    x_axis: xEnt ? parseAxis(asGroup(xEnt, "Scanner/X_Axis")) : null,
    y_axis: yEnt ? parseAxis(asGroup(yEnt, "Scanner/Y_Axis")) : null,
    z_axis: zEnt ? parseAxis(asGroup(zEnt, "Scanner/Z_Axis")) : null,
    focus: focusEnt ? parseAxis(asGroup(focusEnt, "Scanner/Focus")) : null,
    invert_actual_x: attrBool(a, "Invert_Actual_X") ?? false,
    invert_actual_y: attrBool(a, "Invert_Actual_Y") ?? false,
    invert_commanded_x: attrBool(a, "Invert_Commanded_X") ?? false,
    invert_commanded_y: attrBool(a, "Invert_Commanded_Y") ?? false,
  };
}

function parseLightSource(grp: h5wasm.Group): LightSource {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, "Manufacturer"),
    model: attrStrReq(a, "Model"),
    serial_number: attrStrReq(a, "Serial_Number"),
    wavelength: attrFloat(a, "Light_Wavelength"),
    wavelength_unit: attrStr(a, "Light_Wavelength_unit"),
    power_max_nominal: attrFloat(a, "Power_Max_Nominal"),
    power_max_nominal_unit: attrStr(a, "Power_Max_Nominal_unit"),
    power_max_actual: attrFloat(a, "Power_Max_Actual"),
    power_max_actual_unit: attrStr(a, "Power_Max_Actual_unit"),
    power_min_actual: attrFloat(a, "Power_Min_Actual"),
    power_min_actual_unit: attrStr(a, "Power_Min_Actual_unit"),
    power_min_nominal: attrFloat(a, "Power_Min_Nominal"),
    power_min_nominal_unit: attrStr(a, "Power_Min_Nominal_unit"),
    power_bit_resolution: attrFloat(a, "Power_Bit_Resolution"),
    power_bit_resolution_unit: attrStr(a, "Power_Bit_Resolution_unit"),
    power_characterization:
      forwardPowerCharacterizationPoints(
        attrStr(a, "Watts_To_Volts_Algorithm"),
        attrStr(a, "Watts_To_Volts_Params"),
      ) ?? undefined,
  };
}

function parseCollimator(grp: h5wasm.Group): Collimator {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, "Manufacturer"),
    model: attrStrReq(a, "Model"),
    serial_number: attrStrReq(a, "Serial_Number"),
    focal_length: attrFloat(a, "Focal_Length"),
    focal_length_unit: attrStr(a, "Focal_Length_unit"),
  };
}

function parseScannerCard(grp: h5wasm.Group): ScannerCard {
  const a = grp.attrs;
  return {
    manufacturer: attrStrReq(a, "Manufacturer"),
    model: attrStrReq(a, "Model"),
    serial_number: attrStrReq(a, "Serial_Number"),
    communication_protocol: attrStr(a, "Communication_Protocol"),
    sample_period: attrFloat(a, "Sample_Period"),
    sample_period_unit: attrStr(a, "Sample_Period_unit"),
  };
}

function parseSfcf(ds: h5wasm.Dataset, includeBinary: boolean): ScanFieldCorrectionFile {
  const a = ds.attrs;
  const sfcf: ScanFieldCorrectionFile = {
    document_name: attrStrReq(a, "document_name"),
    document_id: attrStrReq(a, "document_id"),
    file_size: attrInt(a, "file_size") ?? 0,
    valid_as_of_date: attrStrReq(a, "valid_as_of_date"),
    document_created_at: attrStr(a, "document_created_at"),
    document_type: attrStr(a, "document_type"),
    original_uri: attrStr(a, "original_uri"),
  };

  if (includeBinary) {
    const raw = ds.value;
    sfcf.raw_bytes =
      raw instanceof Uint8Array
        ? Buffer.from(raw).toString("base64")
        : null;
  }

  return sfcf;
}

function parseOpticalTrain(
  f: h5wasm.File,
  trainId: string,
  includeBinary: boolean,
): OpticalTrain {
  const base = layout.trainPathById(trainId);
  const trainGrp = asGroup(f.get(base), base);
  const a = trainGrp.attrs;

  const sfcfEnt = trainGrp.get("scan_field_correction_file");

  const sfcf: ScanFieldCorrectionFile | null = sfcfEnt instanceof h5wasm.Dataset
    ? parseSfcf(sfcfEnt, includeBinary)
    : null;

  return {
    train_id: trainId,
    id: attrStr(a, "ID"),
    beam_profile_type: attrStr(a, "Beam_Profile_Type"),
    beam_waist_definition: attrStr(a, "Beam_Waist_Definition"),
    beam_waist_major: attrFloat(a, "Beam_Waist_Major"),
    beam_waist_major_unit: attrStr(a, "Beam_Waist_Major_unit"),
    beam_waist_minor: attrFloat(a, "Beam_Waist_Minor"),
    beam_waist_minor_unit: attrStr(a, "Beam_Waist_Minor_unit"),
    beam_waist_offset_z: attrFloat(a, "Beam_Waist_Offset_Z"),
    beam_waist_offset_z_unit: attrStr(a, "Beam_Waist_Offset_Z_unit"),
    build_plane_offset_major: attrFloat(a, "Build_Plane_Offset_Major"),
    build_plane_offset_major_unit: attrStr(a, "Build_Plane_Offset_Major_unit"),
    build_plane_offset_minor: attrFloat(a, "Build_Plane_Offset_Minor"),
    build_plane_offset_minor_unit: attrStr(a, "Build_Plane_Offset_Minor_unit"),
    collimator_focal_length: attrFloat(a, "Collimator_Focal_Length"),
    collimator_focal_length_unit: attrStr(a, "Collimator_Focal_Length_unit"),
    m2_major: attrFloat(a, "M2_Major"),
    m2_minor: attrFloat(a, "M2_Minor"),
    major_axis_angle: attrFloat(a, "Major_Axis_Angle"),
    major_axis_angle_unit: attrStr(a, "Major_Axis_Angle_unit"),
    rayleigh_length_major: attrFloat(a, "Rayleigh_Length_Major"),
    rayleigh_length_major_unit: attrStr(a, "Rayleigh_Length_Major_unit"),
    rayleigh_length_minor: attrFloat(a, "Rayleigh_Length_Minor"),
    rayleigh_length_minor_unit: attrStr(a, "Rayleigh_Length_Minor_unit"),
    scanner_number: attrStr(a, "Scanner_Number"),
    thermal_lensing_passed: attrBool(a, "Thermal_Lensing_Test_Passed"),
    thermal_lensing_focal_plane_shift: attrFloat(a, "Thermal_Lensing_Focal_Plane_Shift"),
    thermal_lensing_focal_plane_shift_unit: attrStr(a, "Thermal_Lensing_Focal_Plane_Shift_unit"),
    thermal_lensing_threshold: attrFloat(a, "Thermal_Lensing_Threshold"),
    thermal_lensing_threshold_unit: attrStr(a, "Thermal_Lensing_Threshold_unit"),
    scanner: parseScanner(asGroup(trainGrp.get("Scanner"), `${base}/Scanner`)),
    light_source: parseLightSource(asGroup(trainGrp.get("Light_Source"), `${base}/Light_Source`)),
    collimator: parseCollimator(asGroup(trainGrp.get("Collimator"), `${base}/Collimator`)),
    scanner_card: parseScannerCard(asGroup(trainGrp.get("Scanner_Card"), `${base}/Scanner_Card`)),
    scan_field_correction_file: sfcf,
  };
}

function parseFile(f: h5wasm.File, opts: ReadOptions): MachineConfig {
  const includeBinary = opts.includeBinary ?? false;

  const meta = parseMeta(f);
  const machine = parseMachine(f);

  const trainsGrp = asGroup(f.get(layout.ROOT_OPTICAL_TRAINS), layout.ROOT_OPTICAL_TRAINS);
  const trainIds = layout.trainIds(trainsGrp.keys());

  const optical_trains = trainIds.map((tid) =>
    parseOpticalTrain(f, tid, includeBinary),
  );

  // Each train's ClearBox group is read verbatim, then shimmed into the v1.1-layout extension tree.
  const clearboxTrains: Array<[string, ExtensionTree]> = [];
  for (const tid of trainIds) {
    const grp = f.get(layout.clearboxPathById(tid));
    if (grp instanceof h5wasm.Group) clearboxTrains.push([tid, readExtensionTree(grp, tid)]);
  }
  const extensions: Record<string, ExtensionTree> = {};
  const clearbox = clearboxFromV1_0Trains(clearboxTrains);
  if (clearbox !== null) extensions[CLEARBOX_EXTENSION_NAME] = clearbox;
  // v1.0 stores the TM_OPCUA extension at the root as `OPCUA/`.
  const opcuaGrp = f.get(layout.ROOT_OPCUA);
  if (opcuaGrp instanceof h5wasm.Group) {
    extensions[layout.LEGACY_OPCUA_EXTENSION_NAME] = readExtensionTree(opcuaGrp, layout.LEGACY_OPCUA_EXTENSION_NAME);
  }

  const config: MachineConfig = { meta, machine, optical_trains };
  if (Object.keys(extensions).length > 0) config.extensions = extensions;
  return config;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export class Hdf5AdapterV1_0 {
  private readonly _path: string;

  constructor(filePath: string) {
    this._path = normPath(filePath);
  }

  async parse(options: ReadOptions = {}): Promise<MachineConfig> {
    await ensureReady();
    const f = new h5wasm.File(this._path, "r");
    try {
      return parseFile(f, options);
    } finally {
      f.close();
    }
  }

  /** Convenience: parse and serialise to canonical JSON. */
  async toJson(options: ToJsonOptions = {}): Promise<string> {
    const config = await this.parse(options);
    const indent = options.indent ?? 2;
    return JSON.stringify(withExtensionsAsJson(config), omitFalseInvertFlags, indent || undefined);
  }

  /**
   * Return all HDF5 attributes at an arbitrary path as a plain record.
   * Returns an empty object (not an error) if the path does not exist.
   * Mirrors Python's `get_raw_group()` and Rust's `get_raw_group()`.
   *
   * @example
   * // OPCUA client attributes (reference_config_opcua.h5 only)
   * const attrs = await reader.getRawGroup('OPCUA/Client');
   * console.log(attrs['Server_URL']);  // e.g. "opc.tcp://..."
   */
  async getRawGroup(hdfPath: string): Promise<Record<string, unknown>> {
    await ensureReady();
    const f = new h5wasm.File(this._path, "r");
    try {
      const entity = f.get(hdfPath);
      if (entity == null) return {};
      if (!(entity instanceof h5wasm.Group) && !(entity instanceof h5wasm.Dataset)) return {};
      return attrExtra(entity.attrs, new Set());
    } finally {
      f.close();
    }
  }
}

