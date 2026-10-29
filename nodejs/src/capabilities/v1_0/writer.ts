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
  ScanFieldCorrectionFile,
} from "../../models.js";
import { backwardFlatFieldsPoints } from "../../powerCharacterization.js";
import * as layout from "./layout.js";
import { assertValidExtensionTree, type ExtensionTree } from "../../extensions/contract.js";
import { writeExtensionTree } from "../../extensions/hdf5.js";
import { CLEARBOX_EXTENSION_NAME, clearboxToV1_0Trains } from "../../clearboxLegacy.js";

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

/**
 * Writes an int64 attribute (`1`) only when `val` is `true`; writes nothing
 * at all when `false` — unlike `wb`, there is no "absent" placeholder
 * written for the `false` case. Used for Scanner's four `invert_*` fields,
 * which never appear in any output unless `true` (user-confirmed,
 * 2026-08-21): a write→read round-trip is deliberately lossy for an
 * explicit `false`, which becomes indistinguishable from "never set".
 */
function wbIfTrue(grp: h5wasm.Group, key: string, val: boolean): void {
  if (val) {
    grp.create_attribute(key, 1, [], "<i8");
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
  ws(grp, "Working_Distance_unit", s.working_distance_unit);
  wf(grp, "Scan_Field_Size_X", s.scan_field_x);
  ws(grp, "Scan_Field_Size_X_unit", s.scan_field_x_unit);
  wf(grp, "Scan_Field_Size_Y", s.scan_field_y);
  ws(grp, "Scan_Field_Size_Y_unit", s.scan_field_y_unit);
  wf(grp, "Scan_Field_Size_Z", s.scan_field_z);
  ws(grp, "Scan_Field_Size_Z_unit", s.scan_field_z_unit);
  wf(grp, "Scan_Head_Offset_X", s.scan_head_offset_x);
  ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit);
  wf(grp, "Scan_Head_Offset_Y", s.scan_head_offset_y);
  ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit);
  wf(grp, "Scan_Head_Offset_Z", s.scan_head_offset_z);
  ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit);
  wf(grp, "Scan_Head_Rotation", s.scan_head_rotation);
  ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit);
  ws(grp, "Axis_Configuration", s.axis_configuration);
  wbIfTrue(grp, "Invert_Actual_X", s.invert_actual_x);
  wbIfTrue(grp, "Invert_Actual_Y", s.invert_actual_y);
  wbIfTrue(grp, "Invert_Commanded_X", s.invert_commanded_x);
  wbIfTrue(grp, "Invert_Commanded_Y", s.invert_commanded_y);
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
  ws(grp, "Light_Wavelength_unit", ls.wavelength_unit);
  wf(grp, "Power_Max_Nominal", ls.power_max_nominal);
  ws(grp, "Power_Max_Nominal_unit", ls.power_max_nominal_unit);
  wf(grp, "Power_Max_Actual", ls.power_max_actual);
  ws(grp, "Power_Max_Actual_unit", ls.power_max_actual_unit);
  wf(grp, "Power_Min_Actual", ls.power_min_actual);
  ws(grp, "Power_Min_Actual_unit", ls.power_min_actual_unit);
  wf(grp, "Power_Min_Nominal", ls.power_min_nominal);
  ws(grp, "Power_Min_Nominal_unit", ls.power_min_nominal_unit);
  // Power_Bit_Resolution is stored as a string in all real HDF5 files.
  ws(grp, "Power_Bit_Resolution",
    ls.power_bit_resolution != null ? String(ls.power_bit_resolution) : null);
  ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit);
  const [wattsAlgorithm, wattsParams] = backwardFlatFieldsPoints(ls.power_characterization);
  ws(grp, "Watts_To_Volts_Algorithm", wattsAlgorithm);
  ws(grp, "Watts_To_Volts_Params", wattsParams);
}

function writeCollimator(grp: h5wasm.Group, c: Collimator): void {
  ws(grp, "Manufacturer", c.manufacturer);
  ws(grp, "Model", c.model);
  ws(grp, "Serial_Number", c.serial_number);
  wf(grp, "Focal_Length", c.focal_length);
  ws(grp, "Focal_Length_unit", c.focal_length_unit);
}

function writeScannerCard(grp: h5wasm.Group, sc: ScannerCard): void {
  ws(grp, "Manufacturer", sc.manufacturer);
  ws(grp, "Model", sc.model);
  ws(grp, "Serial_Number", sc.serial_number);
  ws(grp, "Communication_Protocol", sc.communication_protocol);
  wf(grp, "Sample_Period", sc.sample_period);
  ws(grp, "Sample_Period_unit", sc.sample_period_unit);
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
  clearbox: ExtensionTree | undefined,
): void {
  const tid = layout.trainId(trainIdx);
  const trainGrp = trainsGrp.create_group(tid);

  ws(trainGrp, "ID", train.id);
  ws(trainGrp, "Beam_Profile_Type", train.beam_profile_type);
  ws(trainGrp, "Beam_Waist_Definition", train.beam_waist_definition);
  wf(trainGrp, "Beam_Waist_Major", train.beam_waist_major);
  ws(trainGrp, "Beam_Waist_Major_unit", train.beam_waist_major_unit);
  wf(trainGrp, "Beam_Waist_Minor", train.beam_waist_minor);
  ws(trainGrp, "Beam_Waist_Minor_unit", train.beam_waist_minor_unit);
  wf(trainGrp, "Beam_Waist_Offset_Z", train.beam_waist_offset_z);
  ws(trainGrp, "Beam_Waist_Offset_Z_unit", train.beam_waist_offset_z_unit);
  wf(trainGrp, "Build_Plane_Offset_Major", train.build_plane_offset_major);
  ws(trainGrp, "Build_Plane_Offset_Major_unit", train.build_plane_offset_major_unit);
  wf(trainGrp, "Build_Plane_Offset_Minor", train.build_plane_offset_minor);
  ws(trainGrp, "Build_Plane_Offset_Minor_unit", train.build_plane_offset_minor_unit);
  wf(trainGrp, "Collimator_Focal_Length", train.collimator_focal_length);
  ws(trainGrp, "Collimator_Focal_Length_unit", train.collimator_focal_length_unit);
  wf(trainGrp, "M2_Major", train.m2_major);
  wf(trainGrp, "M2_Minor", train.m2_minor);
  wf(trainGrp, "Major_Axis_Angle", train.major_axis_angle);
  ws(trainGrp, "Major_Axis_Angle_unit", train.major_axis_angle_unit);
  wf(trainGrp, "Rayleigh_Length_Major", train.rayleigh_length_major);
  ws(trainGrp, "Rayleigh_Length_Major_unit", train.rayleigh_length_major_unit);
  wf(trainGrp, "Rayleigh_Length_Minor", train.rayleigh_length_minor);
  ws(trainGrp, "Rayleigh_Length_Minor_unit", train.rayleigh_length_minor_unit);
  ws(trainGrp, "Scanner_Number", train.scanner_number);
  wb(trainGrp, "Thermal_Lensing_Test_Passed", train.thermal_lensing_passed);
  wf(trainGrp, "Thermal_Lensing_Focal_Plane_Shift", train.thermal_lensing_focal_plane_shift);
  ws(trainGrp, "Thermal_Lensing_Focal_Plane_Shift_unit",
    train.thermal_lensing_focal_plane_shift_unit);
  wf(trainGrp, "Thermal_Lensing_Threshold", train.thermal_lensing_threshold);
  ws(trainGrp, "Thermal_Lensing_Threshold_unit", train.thermal_lensing_threshold_unit);

  writeScanner(trainGrp.create_group("Scanner"), train.scanner);
  writeLightSource(trainGrp.create_group("Light_Source"), train.light_source);
  writeCollimator(trainGrp.create_group("Collimator"), train.collimator);
  writeScannerCard(trainGrp.create_group("Scanner_Card"), train.scanner_card);

  if (clearbox !== undefined) {
    const optGrp = trainGrp.create_group(layout.GROUP_OPTIONAL_COMPONENTS);
    writeExtensionTree(optGrp, layout.GROUP_CLEARBOX, clearbox);
  }

  if (train.scan_field_correction_file != null) {
    writeSfcf(trainGrp, train.scan_field_correction_file);
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * File_Version 1.0 HDF5 writer. Public MachineConfigWriter dispatches here.
 */
export class Hdf5WriterV1_0 {
  constructor(private readonly config: MachineConfig) {}

  async write(outputPath: string): Promise<void> {
    const extensions = Object.entries(this.config.extensions ?? {});
    for (const [name, tree] of extensions) {
      assertValidExtensionTree(tree, name);
      if (name !== layout.LEGACY_OPCUA_EXTENSION_NAME && name !== CLEARBOX_EXTENSION_NAME) {
        throw new Error(`File_Version 1.0 has no Extensions group; cannot write extension "${name}"`);
      }
    }
    // Resolved before the file is created, so a rejected config never leaves a partial file.
    const clearboxTree = this.config.extensions?.[CLEARBOX_EXTENSION_NAME];
    const clearboxGroups = clearboxTree === undefined ? {} : clearboxToV1_0Trains(clearboxTree);
    const knownTrains = this.config.optical_trains.map((_, i) => layout.trainId(i));
    const unknown = Object.keys(clearboxGroups).find((id) => !knownTrains.includes(id));
    if (unknown !== undefined) {
      throw new Error(`ClearBox: no optical train "${unknown}" in this configuration`);
    }
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
      ws(machineGrp, "Build_Plate_X_Dimension_unit", ma.build_plate_x_unit);
      wf(machineGrp, "Build_Plate_Y_Dimension", ma.build_plate_y);
      ws(machineGrp, "Build_Plate_Y_Dimension_unit", ma.build_plate_y_unit);
      wf(machineGrp, "Build_Plate_Z_Dimension", ma.build_plate_z);
      ws(machineGrp, "Build_Plate_Z_Dimension_unit", ma.build_plate_z_unit);
      wf(machineGrp, "Build_Plate_Corner_Radius", ma.build_plate_radius);
      ws(machineGrp, "Build_Plate_Corner_Radius_unit", ma.build_plate_radius_unit);
      ws(machineGrp, "Gas_Flow_Direction", ma.gas_flow_direction);
      ws(machineGrp, "Recoat_Direction", ma.recoat_direction);
      ws(machineGrp, "Recoater_Blade_Type", ma.recoater_blade_type);

      // Optical trains
      const trainsGrp = machineGrp.create_group("Optical_Trains");
      for (let i = 0; i < this.config.optical_trains.length; i++) {
        writeOpticalTrain(trainsGrp, i, this.config.optical_trains[i], clearboxGroups[layout.trainId(i)]);
      }

      // v1.0 stores the TM_OPCUA extension at the root as `OPCUA/`.
      const opcua = this.config.extensions?.[layout.LEGACY_OPCUA_EXTENSION_NAME];
      if (opcua !== undefined) writeExtensionTree(f, layout.ROOT_OPCUA, opcua);
    } finally {
      f.close();
    }
  }
}
