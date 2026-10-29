/**
 * File_Version 1.1 HDF5 reader — on-disk layout, attribute names, and casting.
 * Public MachineConfigReader peeks File_Version then dispatches here.
 *
 * Deliberately independent of the previous version's adapter — every helper
 * used here (type-conversion, sub-group parsing) is defined fresh in this
 * module, even where the on-disk shape happens to match today (Collimator,
 * Scanner_Card, sfcf, the
 * low-level attribute-reading helpers). This module could change or be
 * removed later without affecting anything else.
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
  PowerCharacterization,
  EquationConstant,
  CalibrationPoint,
} from "../../models.js";

import * as layout from "./layout.js";
import type { ExtensionTree } from "../../extensions/contract.js";
import { readExtensionTree } from "../../extensions/hdf5.js";
import { withExtensionsAsJson } from "../../extensions/json.js";

export interface ReadOptions {
  /** Include the `.fc3` bytes (`raw_bytes`) in the output. Extension datasets are always complete. */
  includeBinary?: boolean;
}

export interface ToJsonOptions extends ReadOptions {
  /** JSON indentation spaces (default 2). Pass 0 for compact. */
  indent?: number;
}

// ---------------------------------------------------------------------------
// WASM init — lazy singleton, module-local (not shared with any other
// version's adapter).
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
// Attribute reading helpers — independent copies of the equivalent helpers
// used by the previous version's adapter (see module docstring).
// ---------------------------------------------------------------------------

function attrRaw(attrs: H5Attrs, key: string): string | number | boolean | null {
  const attr = attrs[key];
  if (attr == null) return null;
  const v = attr.value;
  if (v == null) return null;
  if (typeof v === "bigint") return Number(v);
  if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") return v;
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

function attrStr(attrs: H5Attrs, key: string): string | null {
  const v = attrRaw(attrs, key);
  if (v == null) return null;
  const s = String(v).trim();
  return s || null;
}

function attrStrReq(attrs: H5Attrs, key: string): string {
  return attrStr(attrs, key) ?? "";
}

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
 * value is exactly `false` — same "omit unless true" business rule applied
 * throughout this project. Also unconditionally drops `is_valid`
 * (`MachineConfigMeta`) — reader-computed, in-memory only, never part of the
 * interop contract; there's no separate dict-builder layer in this language
 * to simply not populate it, so it must be actively stripped here instead.
 */
const INVERT_FLAG_KEYS = new Set([
  "invert_actual_x", "invert_actual_y", "invert_commanded_x", "invert_commanded_y",
]);
function omitFalseInvertFlags(key: string, value: unknown): unknown {
  if (key === "is_valid") return undefined;
  if (INVERT_FLAG_KEYS.has(key) && value === false) return undefined;
  return value;
}

/**
 * Read a named compound dataset of `{name, value}` rows (e.g.
 * `Derivation_Equation_Constants`), defaulting to `[]` if the dataset is
 * absent. h5wasm returns a compound dataset's `.value` as Array-of-
 * Structures rows in declaration order, mapped back to a named object
 * positionally, not by column name.
 */
function readEquationConstants(grp: h5wasm.Group, datasetName: string): EquationConstant[] {
  const ent = grp.get(datasetName);
  if (!(ent instanceof h5wasm.Dataset)) return [];
  const rows = ent.value;
  if (!Array.isArray(rows)) return [];
  return rows.map((row) => {
    const [name, value] = row as unknown as [string, number];
    return { name: String(name), value: Number(value) };
  });
}

/**
 * Read a named compound dataset of `{input_value, output_value}` rows (e.g.
 * `Calibration_Points` or `Characterization_Points`), defaulting to `[]` if
 * the dataset is absent.
 */
function readPointPairs(grp: h5wasm.Group, datasetName: string): CalibrationPoint[] {
  const ent = grp.get(datasetName);
  if (!(ent instanceof h5wasm.Dataset)) return [];
  const rows = ent.value;
  if (!Array.isArray(rows)) return [];
  return rows.map((row) => {
    const [input_value, output_value] = row as unknown as [number, number];
    return { input_value: Number(input_value), output_value: Number(output_value) };
  });
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

/**
 * Change 5: Tuning_Parameters/Tuning_Type have no on-disk source — always
 * null here, regardless of what (if anything) is on the group.
 */
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
    tuning_parameters: null,
    tuning_type: null,
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

/** Changes 3/4: shared `Power_Characterization` shape, two different paths. */
function parsePowerCharacterization(grp: h5wasm.Group): PowerCharacterization {
  const a = grp.attrs;
  return {
    algorithm_type: attrStr(a, "Algorithm_Type"),
    algorithm_equation: attrStr(a, "Algorithm_Equation"),
    input_type: attrStr(a, "Input_Type"),
    units_derived_quantity: attrStr(a, "Units_Derived_Quantity"),
    derivation_equation_constants: readEquationConstants(grp, layout.DS_DERIVATION_EQUATION_CONSTANTS),
    characterization_points: readPointPairs(grp, layout.DS_CHARACTERIZATION_POINTS),
  };
}

/** Change 4: Watts_To_Volts_Algorithm/Params have no on-disk source. */
function parseLightSource(grp: h5wasm.Group): LightSource {
  const a = grp.attrs;
  const pcEnt = grp.get(layout.GROUP_POWER_CHARACTERIZATION);
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
    power_characterization: pcEnt instanceof h5wasm.Group ? parsePowerCharacterization(pcEnt) : undefined,
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

  const sfcfEnt = trainGrp.get(layout.DS_SCAN_FIELD_CORRECTION_FILE);

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

  const optical_trains = trainIds.map((tid) => parseOpticalTrain(f, tid, includeBinary));

  const extensions = parseExtensions(f);
  const config: MachineConfig = { meta, machine, optical_trains };
  if (Object.keys(extensions).length > 0) config.extensions = extensions;
  return config;
}

/** Every group under `Extensions/`, verbatim (ClearBox included). */
function parseExtensions(f: h5wasm.File): Record<string, ExtensionTree> {
  const out: Record<string, ExtensionTree> = {};
  const root = f.get(layout.ROOT_EXTENSIONS);
  if (!(root instanceof h5wasm.Group)) return out;
  for (const name of root.keys()) {
    const ent = root.get(name);
    if (!(ent instanceof h5wasm.Group)) {
      throw new Error(`HDF5: expected Group at "${layout.ROOT_EXTENSIONS}/${name}"`);
    }
    out[name] = readExtensionTree(ent, name);
  }
  return out;
}

// ---------------------------------------------------------------------------
// Public reader API
// ---------------------------------------------------------------------------

export class Hdf5AdapterV1_1 {
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

// ---------------------------------------------------------------------------
// Phase 2 clean-up (see the migration implementation plan) removed
// migrateV1ToV1_1/migrateV1_1ToV1 from here — the coefficients/points
// shape-conversion functions they used now live in
// ../../powerCharacterization.js. Per the
// POWER_CHARACTERIZATION_UNIFICATION_PLAN.md follow-on, power_characterization
// is now the only StableModel representation of this concept: this
// directory's writer.ts always writes power_characterization directly, with
// no fallback derivation left to do. See ../../powerCharacterization.js's
// docs for the full cross-version design.
// ---------------------------------------------------------------------------
