import { describe, it, expect, beforeAll } from 'vitest';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as h5wasm from 'h5wasm/node';
import { MachineConfigReader } from '../src/index.js';
import {
  openMachineConfig,
  UnsupportedFileVersion,
} from '../src/capabilities/index.js';
import { validate } from '../src/schema.js';
import { withExtensionsAsJson } from '../src/extensions/json.js';
import type { MachineConfig } from '../src/models.js';
import { clearbox, clearboxRoot, grid, int, text, attr } from './clearboxHelpers.js';

// ---------------------------------------------------------------------------
// Fixture paths (absolute — resolved relative to this test file)
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const SYNTHETIC       = join(__dirname, '../../fixtures/synthetic_2laser.h5');
const REFERENCE       = join(__dirname, '../../fixtures/reference_config.h5');
const REFERENCE_OPCUA = join(__dirname, '../../fixtures/reference_config_opcua.h5');
const REFERENCE_SENSORS = join(__dirname, '../../fixtures/reference_config_synchronous_sensors.h5');
const REFERENCE_OPCUA_SENSORS = join(
  __dirname, '../../fixtures/reference_config_opcua_synchronous_sensors.h5'
);

// ---------------------------------------------------------------------------
// Shared parsed configs — each fixture opened once for the whole suite
// ---------------------------------------------------------------------------

let synthetic: MachineConfig;
let reference: MachineConfig;
let referenceOpcua: MachineConfig;
let referenceSensors: MachineConfig;
let referenceOpcuaSensors: MachineConfig;

beforeAll(async () => {
  [synthetic, reference, referenceOpcua, referenceSensors, referenceOpcuaSensors] =
    await Promise.all([
      new MachineConfigReader(SYNTHETIC).parse(),
      new MachineConfigReader(REFERENCE).parse(),
      new MachineConfigReader(REFERENCE_OPCUA).parse(),
      new MachineConfigReader(REFERENCE_SENSORS).parse(),
      new MachineConfigReader(REFERENCE_OPCUA_SENSORS).parse(),
    ]);
}, 60_000);   // generous timeout to cover h5wasm WASM init on a cold run

// ===========================================================================
// Meta
// ===========================================================================

describe('MachineConfigReader — meta', () => {
  it('schema_version is "v1" (synthetic)', () => {
    expect(synthetic.meta.schema_version).toBe('v1');
  });

  it('machine_name is non-empty (synthetic)', () => {
    expect(synthetic.meta.machine_name).toBeTruthy();
  });

  it('configuration_hash is exactly 64 hex characters (synthetic)', () => {
    expect(synthetic.meta.configuration_hash).toHaveLength(64);
    expect(synthetic.meta.configuration_hash).toMatch(/^[0-9a-f]{64}$/i);
  });

  it('configuration_hash is exactly 64 hex characters (reference)', () => {
    expect(reference.meta.configuration_hash).toHaveLength(64);
    expect(reference.meta.configuration_hash).toMatch(/^[0-9a-f]{64}$/i);
  });

  it('machine_name matches real AconityMIDI fixture (reference)', () => {
    expect(reference.meta.machine_name).toBe('TM-LPBF-02: AconityMIDI+_OG');
  });

  it('extra is a plain object, may be empty (synthetic)', () => {
    expect(typeof synthetic.meta.extra).toBe('object');
    expect(synthetic.meta.extra).not.toBeNull();
  });
});

// ===========================================================================
// Machine geometry
// ===========================================================================

describe('MachineConfigReader — machine', () => {
  it('build_plate_x ≈ 250 mm (synthetic)', () => {
    expect(synthetic.machine.build_plate_x).toBeCloseTo(250.0, 5);
  });

  it('build_plate_y ≈ 250 mm (synthetic)', () => {
    expect(synthetic.machine.build_plate_y).toBeCloseTo(250.0, 5);
  });

  it('build_plate_x_unit is "mm" (synthetic)', () => {
    expect(synthetic.machine.build_plate_x_unit).toBe('mm');
  });

  it('build_plate_x ≈ 250 mm (reference)', () => {
    expect(reference.machine.build_plate_x).toBeCloseTo(250.0, 5);
  });

  it('build_plate_y ≈ 250 mm (reference)', () => {
    expect(reference.machine.build_plate_y).toBeCloseTo(250.0, 5);
  });
});

// ===========================================================================
// Optical trains — counts & IDs
// ===========================================================================

describe('MachineConfigReader — optical trains', () => {
  it('returns 2 optical trains (synthetic)', () => {
    expect(synthetic.optical_trains).toHaveLength(2);
  });

  it('returns 2 optical trains (reference)', () => {
    expect(reference.optical_trains).toHaveLength(2);
  });

  it('every train has a non-empty train_id (synthetic)', () => {
    for (const train of synthetic.optical_trains) {
      expect(train.train_id).toBeTruthy();
    }
  });

  it('optional_components no longer exists on a train (synthetic)', () => {
    for (const train of synthetic.optical_trains) {
      expect(train).not.toHaveProperty('optional_components');
    }
  });

  it('ClearBox is exposed as an extension with one child per train (reference)', () => {
    expect(Object.keys(clearboxRoot(reference).children)).toEqual(['Optical_Train_01', 'Optical_Train_02']);
  });
});

// ===========================================================================
// Scanner
// ===========================================================================

describe('MachineConfigReader — scanner', () => {
  it('working_distance ≈ 670 mm on train 0 (synthetic)', () => {
    expect(synthetic.optical_trains[0].scanner.working_distance).toBeCloseTo(670.0, 3);
  });

  it('working_distance ≈ 670 mm on train 0 (reference)', () => {
    expect(reference.optical_trains[0].scanner.working_distance).toBeCloseTo(670.0, 3);
  });

  it('working_distance_unit is "mm" on train 0 (reference)', () => {
    expect(reference.optical_trains[0].scanner.working_distance_unit).toBe('mm');
  });

  it('scan_head_offset_x ≈ −87.5 mm on train 0 (reference)', () => {
    expect(reference.optical_trains[0].scanner.scan_head_offset_x).toBeCloseTo(-87.5, 3);
  });

  it('scan_head_offset_y ≈ 23.5 mm on train 0 (reference)', () => {
    expect(reference.optical_trains[0].scanner.scan_head_offset_y).toBeCloseTo(23.5, 3);
  });

  it('scan_head_offset_x ≈ 86.074 mm on train 1 (reference)', () => {
    expect(reference.optical_trains[1].scanner.scan_head_offset_x).toBeCloseTo(86.074, 2);
  });

  it('scan_head_offset_y ≈ −21.695 mm on train 1 (reference)', () => {
    expect(reference.optical_trains[1].scanner.scan_head_offset_y).toBeCloseTo(-21.695, 2);
  });

  it('scan_head_rotation is 0° on train 0 (reference)', () => {
    expect(reference.optical_trains[0].scanner.scan_head_rotation).toBeCloseTo(0.0, 3);
  });

  it('scan_head_rotation is 180° on train 1 (reference)', () => {
    expect(reference.optical_trains[1].scanner.scan_head_rotation).toBeCloseTo(180.0, 3);
  });

  it('scan_head_rotation_unit is "degrees" (reference)', () => {
    expect(reference.optical_trains[0].scanner.scan_head_rotation_unit).toBe('degrees');
  });

  it('scanner serial_number is non-empty on every train (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.scanner.serial_number).toBeTruthy();
    }
  });

  it('invert_* flags default to false when absent from the fixture (reference)', () => {
    const s = reference.optical_trains[0].scanner;
    expect(s.invert_actual_x).toBe(false);
    expect(s.invert_actual_y).toBe(false);
    expect(s.invert_commanded_x).toBe(false);
    expect(s.invert_commanded_y).toBe(false);
  });

  it('invert_* flags are omitted from JSON output when false (reference)', async () => {
    const json = JSON.parse(await new MachineConfigReader(REFERENCE).toJson());
    const scanner = json.optical_trains[0].scanner;
    expect('invert_actual_x' in scanner).toBe(false);
    expect('invert_actual_y' in scanner).toBe(false);
    expect('invert_commanded_x' in scanner).toBe(false);
    expect('invert_commanded_y' in scanner).toBe(false);
  });
});

// ===========================================================================
// Collimator
// ===========================================================================

describe('MachineConfigReader — collimator', () => {
  it('focal_length is 120 mm on every train (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.collimator.focal_length).toBeCloseTo(120.0, 3);
    }
  });

  it('focal_length_unit is "mm" on every train (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.collimator.focal_length_unit).toBe('mm');
    }
  });
});

// ===========================================================================
// Light source
// ===========================================================================

describe('MachineConfigReader — light_source', () => {
  it('wavelength_unit is "nm" on train 0 (reference)', () => {
    expect(reference.optical_trains[0].light_source.wavelength_unit).toBe('nm');
  });
});

// ===========================================================================
// Scanner card
// ===========================================================================

describe('MachineConfigReader — scanner_card', () => {
  it('model is "SP-ICE-3" on every train (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.scanner_card.model).toBe('SP-ICE-3');
    }
  });

  it('sample_period_unit is "μs" on every train (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.scanner_card.sample_period_unit).toBe('μs');
    }
  });
});

// ===========================================================================
// Thermal lensing
// ===========================================================================

describe('MachineConfigReader — thermal lensing', () => {
  it('thermal_lensing_passed is false on train 0 (reference)', () => {
    expect(reference.optical_trains[0].thermal_lensing_passed).toBe(false);
  });

  it('thermal_lensing_passed is true on train 1 (reference)', () => {
    expect(reference.optical_trains[1].thermal_lensing_passed).toBe(true);
  });
});

// ===========================================================================
// ClearBox
// ===========================================================================

describe('MachineConfigReader — ClearBox', () => {
  it('clearbox is present on reference fixture', () => {
    expect(clearbox(reference, 0)).toBeDefined();
  });

  it('clearbox Ip_Address is non-empty (reference)', () => {
    expect(text(clearbox(reference, 0), 'Ip_Address')).toBeTruthy();
  });

  it('clearbox Data_Port is an integer (reference)', () => {
    expect(Number.isInteger(int(clearbox(reference, 0), 'Data_Port'))).toBe(true);
  });

  it('the two shared values live on the extension root, once', () => {
    const root = clearboxRoot(reference);
    expect(text(root, 'Output_Path')).toBeTruthy();
    expect(int(root, 'Software_Trigger_Delay')).not.toBeNull();
    expect(clearbox(reference, 0).attrs).not.toHaveProperty('Output_Path');
  });

  it('v1.0-only attributes ride along on each train', () => {
    expect(clearbox(reference, 0).attrs).toHaveProperty('Show_Console');
    expect(clearbox(reference, 0).attrs).not.toHaveProperty('Volts_To_Watts_Algorithm');
  });

  it('Synchronous_Sensors is absent on the reference fixture', () => {
    expect(clearbox(reference, 0).children).not.toHaveProperty('Synchronous_Sensors');
  });
});

// ===========================================================================
// SynchronousSensor — reference_config_synchronous_sensors.h5, not
// reference_config.h5, which deliberately has no sensors (see
// SYNCHRONOUS_SENSOR_PLAN.md Phase 0).
// ===========================================================================

const sensorsOf = (cfg: MachineConfig) => clearbox(cfg, 0).children['Synchronous_Sensors'].children;

describe('MachineConfigReader — SynchronousSensor', () => {
  it('the "Oxygen Sensor" entry is present under train 0', () => {
    expect(Object.keys(sensorsOf(referenceSensors))).toEqual(['Oxygen Sensor']);
  });

  it('the scalar attributes match the real ZR800 example', () => {
    const s = sensorsOf(referenceSensors)['Oxygen Sensor'];
    expect(text(s, 'Sensor_Name')).toBe('ZR800 Oxygen Analyzer');
    expect(attr(s, 'Sensor_Output_Range_Low')).toBe(-1.0);
    expect(attr(s, 'Sensor_Output_Range_High')).toBe(6.0);
    expect(text(s, 'Sensor_Output_Space')).toBe('log10(ppm)');
    expect(text(s, 'Sensor_Model')).toBe('ZR810');
    expect(text(s, 'Sensor_Manufacturer')).toBe('Industrial Physics');
    expect(text(s, 'Sensor_Scope')).toBe('Global');
    expect(text(s, 'Units_Derived_Quantity')).toBe('ppm');
    expect(int(s, 'Port_ID')).toBe(5);
    expect(text(s, 'Sensor_Type')).toBe('Oxygen Sensor');
    expect(text(s, 'Input_Type')).toBe('4-20 mA');
    expect(text(s, 'Algorithm_Type')).toBe('Log-Linear');
    expect(text(s, 'Algorithm_Equation')).toBe('log(ppm) = a*mA + b');
    expect(text(s, 'Calibration_Source')).toBe('Datasheet');
    expect(attr(s, 'Sample_Period')).toBe(5.0);
    expect(text(s, 'Metadata')).toContain('Synchronous Sensor because Clearbox is responsible');
  });

  it('Derivation_Equation_Constants is a table with both rows in order', () => {
    const t = sensorsOf(referenceSensors)['Oxygen Sensor'].tables!['Derivation_Equation_Constants'];
    expect(t.columns['name']).toEqual(['a', 'b']);
    expect(Array.from(t.columns['value'] as Float64Array)).toEqual([0.4375, -2.75]);
  });

  it('Calibration_Points has both rows in order, satisfying the log-space algorithm equation', () => {
    const s = sensorsOf(referenceSensors)['Oxygen Sensor'];
    const points = s.tables!['Calibration_Points'];
    const inputs = Array.from(points.columns['input_value'] as Float64Array);
    const outputs = Array.from(points.columns['output_value'] as Float64Array);
    expect(inputs).toEqual([4.0, 20.0]);
    expect(outputs).toEqual([-1.0, 6.0]);
    // log(ppm) = a*mA + b: the points are recorded in Sensor_Output_Space (log10(ppm)) units.
    const [a, b] = Array.from(s.tables!['Derivation_Equation_Constants'].columns['value'] as Float64Array);
    inputs.forEach((x, i) => expect(a * x + b).toBeCloseTo(outputs[i], 10));
  });

  it('the combined OPCUA+sensors fixture has both features present at once', () => {
    expect(referenceOpcuaSensors.extensions?.TM_OPCUA).toBeDefined();
    const t = sensorsOf(referenceOpcuaSensors)['Oxygen Sensor'].tables!['Derivation_Equation_Constants'];
    expect(t.columns['name']).toEqual(['a', 'b']);
  });
});

// ===========================================================================
// ScanFieldCorrectionFile
// ===========================================================================

describe('MachineConfigReader — scan_field_correction_file', () => {
  it('file_size on train 0 is 1138799 (reference)', () => {
    expect(reference.optical_trains[0].scan_field_correction_file?.file_size).toBe(1138799);
  });

  it('file_size on train 1 is 1142763 (reference)', () => {
    expect(reference.optical_trains[1].scan_field_correction_file?.file_size).toBe(1142763);
  });

  it('document_name is non-empty (reference)', () => {
    expect(reference.optical_trains[0].scan_field_correction_file?.document_name).toBeTruthy();
  });

  it('document_id is non-empty (reference)', () => {
    expect(reference.optical_trains[0].scan_field_correction_file?.document_id).toBeTruthy();
  });

  it('raw_bytes is absent by default (reference)', () => {
    expect(reference.optical_trains[0].scan_field_correction_file?.raw_bytes).toBeUndefined();
  });
});

// ===========================================================================
// Correction data arrays (requires includeBinary: true)
// ===========================================================================

describe('MachineConfigReader — includeBinary: true', () => {
  let refBinary: MachineConfig;

  beforeAll(async () => {
    refBinary = await new MachineConfigReader(REFERENCE).parse({ includeBinary: true });
  }, 60_000);

  it('raw_bytes is present when includeBinary=true', () => {
    const sfcf = refBinary.optical_trains[0].scan_field_correction_file!;
    expect(sfcf.raw_bytes).toBeTruthy();
  });

  it('the correction grids are always complete, with or without includeBinary', () => {
    expect(grid(reference, 0, 'Correction_Data').data).toHaveLength(257 * 257 * 2);
    expect(grid(refBinary, 0, 'Correction_Data').data).toHaveLength(257 * 257 * 2);
  });
});

// ===========================================================================
// ClearBox correction grids — ordinary extension datasets; they feed the correction-hash CLI.
// ===========================================================================

describe('MachineConfigReader — ClearBox correction grids', () => {
  const cd0 = () => grid(reference, 0, 'Correction_Data');
  const icd0 = () => grid(reference, 0, 'Inverse_Correction_Data');

  /** Byte view of a Float64Array's own data (respects byteOffset/byteLength). */
  const bytesOf = (a: Float64Array) => Buffer.from(a.buffer, a.byteOffset, a.byteLength);

  it('the forward grid is a Float64Array of shape [257, 257, 2]', () => {
    expect(cd0().data).toBeInstanceOf(Float64Array);
    expect(cd0().shape).toEqual([257, 257, 2]);
    expect(cd0().data.length).toBe(257 * 257 * 2);
  });

  it('preserves NaN (border cells are not converted to null/0)', () => {
    expect(Array.from(cd0().data).some((v) => Number.isNaN(v))).toBe(true);
  });

  it('the inverse grid is a Float64Array of shape [257, 257, 2]', () => {
    expect(icd0().data).toBeInstanceOf(Float64Array);
    expect(icd0().shape).toEqual([257, 257, 2]);
  });

  it('forward and inverse correction grids are not identical', () => {
    expect(bytesOf(cd0().data)).not.toEqual(bytesOf(icd0().data));
  });

  it('the first finite inverse value differs from the forward one', () => {
    const firstFwd = Array.from(cd0().data).find((v) => isFinite(v));
    const firstInv = Array.from(icd0().data).find((v) => isFinite(v));
    expect(firstFwd).toBeDefined();
    expect(firstInv).toBeDefined();
    expect(Math.abs(firstInv! - firstFwd!)).toBeGreaterThan(0.0001);
  });

  it('train 1 correction grid differs from train 0', () => {
    expect(bytesOf(cd0().data)).not.toEqual(bytesOf(grid(reference, 1, 'Correction_Data').data));
  });
});

// ===========================================================================
// OPCUA
// ===========================================================================

describe('MachineConfigReader — OPCUA group as an extension tree', () => {
  const tree = () => referenceOpcua.extensions!.TM_OPCUA;
  const val = (t: { attrs: Record<string, { value: unknown }> }, name: string) => t.attrs[name]?.value;

  it('carries only the ClearBox extension for the reference fixture (no OPCUA group)', () => {
    expect(Object.keys(reference.extensions!)).toEqual(['ClearBox']);
  });

  it('core has no typed opcua model', () => {
    expect(referenceOpcua).not.toHaveProperty('opcua');
  });

  it('exposes the legacy OPCUA/ group as extensions.TM_OPCUA', () => {
    expect(Object.keys(referenceOpcua.extensions!).sort()).toEqual(['ClearBox', 'TM_OPCUA']);
    expect(Object.keys(tree().children).sort()).toEqual(['Client', 'Pipe', 'Triggers']);
  });

  it('Client attributes are read verbatim', () => {
    const c = tree().children.Client;
    expect(Number(val(c, 'BFS_Max_Depth'))).toBe(16);
    expect(Number(val(c, 'Publish_Interval'))).toBe(250);
    expect(Number(val(c, 'Sampling_Interval'))).toBe(250);
    expect(Number(val(c, 'Session_Timeout'))).toBe(60000);
    expect(val(c, 'Auth_Mode')).toBe('UsernamePassword');
    expect(val(c, 'Machine_Profile')).toBe('Aconity');
    expect(val(c, 'Queue_Policy')).toBe('DropOldest');
    expect(String(val(c, 'Server_URL')).length).toBeGreaterThan(0);
  });

  it('Pipe attributes are read verbatim', () => {
    const p = tree().children.Pipe;
    expect(Number(val(p, 'Buffer_Size'))).toBe(65536);
    expect(val(p, 'Min_Integrity_Level')).toBe('0x2000');
    expect(val(p, 'Pipe_Name')).toBe('\\\\.\\pipe\\opc_ua_client_pipe');
  });

  it('Triggers group carries its own attributes and one child group per trigger', () => {
    const t = tree().children.Triggers;
    expect(Number(val(t, 'Trigger_Stop_Ceiling_Layers'))).toBe(3);
    expect(Number(val(t, 'Triggers_Enabled'))).toBe(1);
    expect(Object.keys(t.children).sort()).toEqual(['Chamber Oxygen Level', 'Laser Emission Interlock']);
  });

  it('trigger groups keep every attribute', () => {
    const t = tree().children.Triggers.children;
    const laser = t['Laser Emission Interlock'];
    expect(val(laser, 'ID')).toBe('trigger_1');
    expect(val(laser, 'Signal')).toBe('yellow_light');
    expect(val(laser, 'Subsystem')).toBe('Chamber');
    expect(val(laser, 'Event')).toBe('SensorEvents');
    const oxygen = t['Chamber Oxygen Level'];
    expect(val(oxygen, 'Start_Value')).toBe('700');
    expect(val(oxygen, 'Stop_Value')).toBe('1000');
    expect(val(oxygen, 'Component')).toBe('process_chamber::gas_management::oxygen_sensor::1');
  });
});

// ===========================================================================
// JSON serialization
// ===========================================================================

describe('MachineConfigReader — toJson()', () => {
  it('produces valid JSON from the synthetic fixture', async () => {
    const json = await new MachineConfigReader(SYNTHETIC).toJson();
    expect(() => JSON.parse(json)).not.toThrow();
  });

  it('JSON output has top-level keys: meta, machine, optical_trains (synthetic)', async () => {
    const json = await new MachineConfigReader(SYNTHETIC).toJson();
    const parsed = JSON.parse(json);
    expect(parsed).toHaveProperty('meta');
    expect(parsed).toHaveProperty('machine');
    expect(parsed).toHaveProperty('optical_trains');
  });

  it('only the ClearBox extension appears in reference fixture JSON', async () => {
    const json = await new MachineConfigReader(REFERENCE).toJson();
    expect(Object.keys(JSON.parse(json).extensions)).toEqual(['ClearBox']);
  });

  it('the OPCUA group appears under extensions.TM_OPCUA in reference_opcua fixture JSON', async () => {
    const json = await new MachineConfigReader(REFERENCE_OPCUA).toJson();
    expect(JSON.parse(json).extensions).toHaveProperty('TM_OPCUA');
  });

  it('optional_components is absent from JSON', async () => {
    const json = await new MachineConfigReader(REFERENCE).toJson();
    const parsed = JSON.parse(json);
    expect(parsed.optical_trains?.[0]).not.toHaveProperty('optional_components');
  });

  it('produces compact single-line JSON when indent=0', async () => {
    const json = await new MachineConfigReader(SYNTHETIC).toJson({ indent: 0 });
    expect(json).not.toMatch(/\n/);
  });

  it('toJson structure is consistent with parse() output', async () => {
    const reader = new MachineConfigReader(SYNTHETIC);
    const [parsed, json] = await Promise.all([reader.parse(), reader.toJson()]);
    const fromJson = JSON.parse(json);
    expect(fromJson.meta.machine_name).toBe(parsed.meta.machine_name);
    expect(fromJson.optical_trains).toHaveLength(parsed.optical_trains.length);
  });
});

// ===========================================================================
// getRawGroup
// ===========================================================================

describe('MachineConfigReader — getRawGroup()', () => {
  it('returns {} for a path that does not exist', async () => {
    const result = await new MachineConfigReader(REFERENCE).getRawGroup('does/not/exist');
    expect(result).toEqual({});
  });

  it('returns {} for "OPCUA" on reference fixture (no OPCUA group)', async () => {
    const result = await new MachineConfigReader(REFERENCE).getRawGroup('OPCUA');
    expect(result).toEqual({});
  });

  it('"OPCUA/Client" on reference_opcua fixture returns a populated object', async () => {
    const result = await new MachineConfigReader(REFERENCE_OPCUA).getRawGroup('OPCUA/Client');
    expect(Object.keys(result).length).toBeGreaterThan(0);
  });

  it('"OPCUA/Client" includes Server_URL with a non-empty string value (reference_opcua)', async () => {
    const result = await new MachineConfigReader(REFERENCE_OPCUA).getRawGroup('OPCUA/Client');
    expect(result).toHaveProperty('Server_URL');
    expect(typeof result['Server_URL']).toBe('string');
    expect((result['Server_URL'] as string).length).toBeGreaterThan(0);
  });
});

// ===========================================================================
// Unknown File_Version
// ===========================================================================

describe('MachineConfigReader — unknown File_Version', () => {
  it('does not use the v1.0 layout for a 2.0 file with no Machine group', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-fv-'));
    const out = join(dir, 'future.h5').replace(/\\/g, '/');
    try {
      await h5wasm.ready;
      const f = new h5wasm.File(out, 'w');
      f.create_attribute('File_Version', '2.0', [], 'S');
      f.close();

      await expect(new MachineConfigReader(out).parse()).rejects.toThrow(
        UnsupportedFileVersion,
      );
      await expect(new MachineConfigReader(out).parse()).rejects.toThrow(/2\.0/);

      const result = await openMachineConfig(out);
      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.code).toBe('UnsupportedVersion');
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

// ===========================================================================
// Schema validation
// ===========================================================================

describe('MachineConfigReader — schema validation', () => {
  it('synthetic fixture output passes schema validation', () => {
    expect(validate(withExtensionsAsJson(synthetic))).toEqual([]);
  });

  it('reference fixture output passes schema validation', () => {
    expect(validate(withExtensionsAsJson(reference))).toEqual([]);
  });

  it('reference_opcua fixture output passes schema validation', () => {
    expect(validate(withExtensionsAsJson(referenceOpcua))).toEqual([]);
  });
});
