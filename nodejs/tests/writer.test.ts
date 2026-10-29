import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes, createHash } from 'node:crypto';
import * as h5wasm from 'h5wasm/node';
import { MachineConfigWriter, canonicalDigest } from '../src/index.js';
import { MachineConfigReader } from '../src/index.js';
import { UnsupportedFileVersion } from '../src/capabilities/index.js';
import { MockConfigBuilder } from '../src/builder.js';
import type { MachineConfig } from '../src/index.js';
import { addSensor, clearbox, grid, sensorTree, text } from './clearboxHelpers.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE      = join(__dirname, '../../fixtures/reference_config.h5');
const REFERENCE_OPCUA = join(__dirname, '../../fixtures/reference_config_opcua.h5');
const REFERENCE_SENSORS = join(__dirname, '../../fixtures/reference_config_synchronous_sensors.h5');
const SYNTHETIC      = join(__dirname, '../../fixtures/synthetic_2laser.h5');

/** Absolute path to a unique temp HDF5 file that does not exist yet. */
function tmpH5(): string {
  return join(tmpdir(), `test_writer_${randomBytes(8).toString('hex')}.h5`);
}

// Module-level fixture configs, parsed once in beforeAll.
let reference: MachineConfig;
let referenceOpcua: MachineConfig;
let referenceSensors: MachineConfig;
let synthetic: MachineConfig;

// Paths to temp files created by write tests — deleted in afterAll.
const tmpFiles: string[] = [];

beforeAll(async () => {
  [reference, referenceOpcua, referenceSensors, synthetic] = await Promise.all([
    new MachineConfigReader(REFERENCE).parse(),
    new MachineConfigReader(REFERENCE_OPCUA).parse(),
    new MachineConfigReader(REFERENCE_SENSORS).parse(),
    new MachineConfigReader(SYNTHETIC).parse(),
  ]);
}, 60_000);

afterAll(() => {
  for (const p of tmpFiles) {
    if (existsSync(p)) unlinkSync(p);
  }
});

// ---------------------------------------------------------------------------
// Roundtrip helpers
// ---------------------------------------------------------------------------

/**
 * Write `config` to a temp file, read it back, return the re-parsed config.
 * Registers the temp path for cleanup.
 */
async function roundtrip(config: MachineConfig): Promise<MachineConfig> {
  const out = tmpH5();
  tmpFiles.push(out);
  await new MachineConfigWriter(config).write(out);
  return new MachineConfigReader(out).parse();
}

// ---------------------------------------------------------------------------
// Smoke tests
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — class', () => {
  it('is importable', () => {
    expect(MachineConfigWriter).toBeDefined();
  });

  it('can be instantiated', () => {
    expect(new MachineConfigWriter(reference)).toBeInstanceOf(MachineConfigWriter);
  });

  it('rejects an unknown File_Version on the model', () => {
    const bad: MachineConfig = {
      ...reference,
      meta: { ...reference.meta, file_version: '2.0' },
    };
    expect(() => new MachineConfigWriter(bad)).toThrow(UnsupportedFileVersion);
    expect(() => new MachineConfigWriter(bad)).toThrow(/2\.0/);
  });
});

// ---------------------------------------------------------------------------
// Prerequisite fix (CONFIGURATION_HASH_PLAN.md): writers must no longer
// materialize a hardcoded default (e.g. "mm") for an unset `*_unit` field —
// that default was never real data, just the reader's own Rule-8 locked
// constant written early for no benefit, and it broke pre-write/post-write
// hash agreement. One representative field per affected struct, mirroring
// Python's/Rust's equivalent test's shape exactly.
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — unset unit fields round trip as null, not a default', () => {
  it('leaves unset *_unit fields null rather than materializing a default', async () => {
    const built = new MockConfigBuilder({ nLasers: 1 }).build();
    const config: MachineConfig = {
      ...built,
      machine: { ...built.machine, build_plate_radius_unit: null },
      optical_trains: built.optical_trains.map((train) => ({
        ...train,
        major_axis_angle_unit: null,
        scanner: { ...train.scanner, working_distance_unit: null },
        light_source: { ...train.light_source, wavelength_unit: null },
        collimator: { ...train.collimator, focal_length_unit: null },
        scanner_card: { ...train.scanner_card, sample_period_unit: null },
      })),
    };

    const rt = await roundtrip(config);

    expect(rt.machine.build_plate_radius_unit).toBeNull();
    expect(rt.optical_trains[0].major_axis_angle_unit).toBeNull();
    expect(rt.optical_trains[0].scanner.working_distance_unit).toBeNull();
    expect(rt.optical_trains[0].light_source.wavelength_unit).toBeNull();
    expect(rt.optical_trains[0].collimator.focal_length_unit).toBeNull();
    expect(rt.optical_trains[0].scanner_card.sample_period_unit).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Roundtrip: reference fixture
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — reference roundtrip', () => {
  let rt: MachineConfig;

  beforeAll(async () => {
    rt = await roundtrip(reference);
  }, 60_000);

  it('produces a readable HDF5 file', () => {
    expect(rt).toBeDefined();
  });

  it('machine_name survives roundtrip', () => {
    expect(rt.meta.machine_name).toBe(reference.meta.machine_name);
  });

  it('configuration_hash is real and valid after roundtrip', () => {
    // reference_config.h5 is externally-authored, so its own stored hash is
    // expected to be invalid — but once *this* writer produces a file, that
    // file's stored hash must be real (64 hex chars, not a passthrough of
    // the input) and must read back valid.
    expect(rt.meta.configuration_hash).toMatch(/^[0-9a-f]{64}$/);
    expect(rt.meta.configuration_hash).not.toBe(reference.meta.configuration_hash);
    expect(rt.meta.is_valid).toBe(true);
  });

  it('optical train count survives roundtrip', () => {
    expect(rt.optical_trains.length).toBe(reference.optical_trains.length);
  });

  it('build plate X survives roundtrip', () => {
    expect(rt.machine.build_plate_x).toBeCloseTo(reference.machine.build_plate_x!, 5);
  });

  it('recoater_blade_type survives roundtrip', () => {
    expect(rt.machine.recoater_blade_type).toBe(reference.machine.recoater_blade_type);
  });

  it('working distance (train 0) survives roundtrip', () => {
    expect(rt.optical_trains[0].scanner.working_distance)
      .toBeCloseTo(reference.optical_trains[0].scanner.working_distance!, 5);
  });

  it('thermal_lensing_passed (train 0) survives roundtrip', () => {
    expect(rt.optical_trains[0].thermal_lensing_passed)
      .toBe(reference.optical_trains[0].thermal_lensing_passed);
  });

  it('scan_field_correction_file document_name survives roundtrip', () => {
    const orig = reference.optical_trains[0].scan_field_correction_file;
    const back = rt.optical_trains[0].scan_field_correction_file;
    expect(back?.document_name).toBe(orig?.document_name);
  });

  it('scan_field_correction_file file_size survives roundtrip', () => {
    const orig = reference.optical_trains[0].scan_field_correction_file;
    const back = rt.optical_trains[0].scan_field_correction_file;
    expect(back?.file_size).toBe(orig?.file_size);
  });

  it('ClearBox Ip_Address survives roundtrip', () => {
    expect(text(clearbox(rt, 0), 'Ip_Address')).toBe(text(clearbox(reference, 0), 'Ip_Address'));
  });

  it('the whole ClearBox tree survives roundtrip (same canonical digest)', () => {
    expect(canonicalDigest(rt.extensions!.ClearBox)).toBe(canonicalDigest(reference.extensions!.ClearBox));
  });

  it('only the ClearBox extension appears for a fixture without an OPCUA group', () => {
    expect(Object.keys(rt.extensions!)).toEqual(['ClearBox']);
  });

  it('Synchronous_Sensors survives roundtrip as absent', () => {
    expect(clearbox(rt, 0).children).not.toHaveProperty('Synchronous_Sensors');
  });
});

// ---------------------------------------------------------------------------
// Scanner invert_* flags
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — Scanner invert_* flags', () => {
  it('default to false and are omitted from JSON when the fixture has none', async () => {
    const rt = await roundtrip(reference);
    const s = rt.optical_trains[0].scanner;
    expect(s.invert_actual_x).toBe(false);
    expect(s.invert_actual_y).toBe(false);
    expect(s.invert_commanded_x).toBe(false);
    expect(s.invert_commanded_y).toBe(false);
  });

  it('only true values survive as real HDF5 attributes', async () => {
    const cfg: MachineConfig = structuredClone(reference);
    cfg.optical_trains[0].scanner.invert_actual_x = true;
    cfg.optical_trains[0].scanner.invert_actual_y = false;
    cfg.optical_trains[0].scanner.invert_commanded_x = true;
    cfg.optical_trains[0].scanner.invert_commanded_y = false;

    const out = tmpH5();
    tmpFiles.push(out);
    await new MachineConfigWriter(cfg).write(out);

    // Raw HDF5 inspection: only the two true-valued attributes exist at all.
    await h5wasm.ready;
    const f = new h5wasm.File(out, 'r');
    const scannerGrp = f.get('Machine/Optical_Trains/Optical_Train_01/Scanner') as h5wasm.Group;
    const attrNames = Object.keys(scannerGrp.attrs);
    f.close();
    expect(attrNames).toContain('Invert_Actual_X');
    expect(attrNames).toContain('Invert_Commanded_X');
    expect(attrNames).not.toContain('Invert_Actual_Y');
    expect(attrNames).not.toContain('Invert_Commanded_Y');

    // Read path returns the correct value either way.
    const rt = await new MachineConfigReader(out).parse();
    const s = rt.optical_trains[0].scanner;
    expect(s.invert_actual_x).toBe(true);
    expect(s.invert_actual_y).toBe(false);
    expect(s.invert_commanded_x).toBe(true);
    expect(s.invert_commanded_y).toBe(false);

    // JSON output only ever shows the true ones.
    const json = JSON.parse(await new MachineConfigReader(out).toJson());
    const scannerJson = json.optical_trains[0].scanner;
    expect(scannerJson.invert_actual_x).toBe(true);
    expect(scannerJson.invert_commanded_x).toBe(true);
    expect('invert_actual_y' in scannerJson).toBe(false);
    expect('invert_commanded_y' in scannerJson).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Roundtrip: SynchronousSensor
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — SynchronousSensor roundtrip', () => {
  it('the real ZR800 example survives roundtrip with exact compound-dataset values in order', async () => {
    const rt = await roundtrip(referenceSensors);
    const sensor = clearbox(rt, 0).children['Synchronous_Sensors'].children['Oxygen Sensor'];
    expect(text(sensor, 'Sensor_Name')).toBe('ZR800 Oxygen Analyzer');
    const constants = sensor.tables!['Derivation_Equation_Constants'];
    expect(constants.columns['name']).toEqual(['a', 'b']);
    expect(Array.from(constants.columns['value'] as Float64Array)).toEqual([0.4375, -2.75]);
    const points = sensor.tables!['Calibration_Points'];
    expect(Array.from(points.columns['input_value'] as Float64Array)).toEqual([4.0, 20.0]);
    expect(Array.from(points.columns['output_value'] as Float64Array)).toEqual([-1.0, 6.0]);
  });

  it('a sensor that exists but has zero-row compound datasets roundtrips correctly', async () => {
    // The sensor group exists but its compound dataset has zero rows — proves 0-length compound
    // dataset creation/read works, not just that an absent group defaults to empty.
    const cfg: MachineConfig = structuredClone(reference);
    addSensor(cfg, 'Untested Sensor', sensorTree([]));
    const rt = await roundtrip(cfg);
    const sensor = clearbox(rt, 0).children['Synchronous_Sensors'].children['Untested Sensor'];
    expect(sensor.tables!['Derivation_Equation_Constants'].rows).toBe(0);
  });

  it('an arbitrary, differently-styled key survives roundtrip verbatim', async () => {
    const cfg: MachineConfig = structuredClone(reference);
    addSensor(cfg, 'HUMIDITY_SENSOR_2', sensorTree([], { Port_ID: { dtype: 'i64', value: 9n } }));
    const rt = await roundtrip(cfg);
    const sensors = clearbox(rt, 0).children['Synchronous_Sensors'].children;
    expect(Object.keys(sensors)).toContain('HUMIDITY_SENSOR_2');
    expect(Number(sensors.HUMIDITY_SENSOR_2.attrs['Port_ID'].value)).toBe(9);
  });

  it('OPCUA and a newly-added sensor coexist through the writer', async () => {
    // Parses a real OPCUA fixture, adds a sensor purely in memory, writes, and confirms the OPCUA
    // extension tree and the sensor both survive re-reading.
    const cfg: MachineConfig = structuredClone(referenceOpcua);
    expect(cfg.extensions?.TM_OPCUA).toBeDefined();
    addSensor(cfg, 'Oxygen Sensor', sensorTree([['a', 0.4375], ['b', -2.75]]));
    const rt = await roundtrip(cfg);
    expect(canonicalDigest(rt.extensions!.TM_OPCUA)).toBe(canonicalDigest(referenceOpcua.extensions!.TM_OPCUA));
    const sensor = clearbox(rt, 0).children['Synchronous_Sensors'].children['Oxygen Sensor'];
    const constants = sensor.tables!['Derivation_Equation_Constants'];
    expect(constants.columns['name']).toEqual(['a', 'b']);
  });

  it('a constant name too long for the 64-byte fixed-length field is rejected, not truncated', () => {
    const cfg: MachineConfig = structuredClone(reference);
    addSensor(cfg, 'Oversized Name Sensor', sensorTree([['a'.repeat(65), 1.0]]));
    // The writer hashes the config on construction, which already validates every tree.
    expect(() => new MachineConfigWriter(cfg)).toThrow(/do not fit/);
  });
});

// ---------------------------------------------------------------------------
// Roundtrip: OPC-UA fixture
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — OPC-UA extension roundtrip', () => {
  let rt: MachineConfig;

  beforeAll(async () => {
    rt = await roundtrip(referenceOpcua);
  }, 60_000);

  const val = (t: { attrs: Record<string, { value: unknown }> }, name: string) => t.attrs[name]?.value;

  it('the TM_OPCUA tree is preserved verbatim (same canonical digest)', () => {
    expect(rt.extensions).toBeDefined();
    expect(canonicalDigest(rt.extensions!.TM_OPCUA)).toBe(canonicalDigest(referenceOpcua.extensions!.TM_OPCUA));
  });

  it('there is no typed opcua model on either side', () => {
    expect(rt).not.toHaveProperty('opcua');
    expect(referenceOpcua).not.toHaveProperty('opcua');
  });

  it('client, pipe and trigger attributes survive with their dtype', () => {
    const orig = referenceOpcua.extensions!.TM_OPCUA;
    const back = rt.extensions!.TM_OPCUA;
    expect(back.children.Client.attrs.Server_URL).toEqual(orig.children.Client.attrs.Server_URL);
    expect(back.children.Client.attrs.Session_Timeout).toEqual(orig.children.Client.attrs.Session_Timeout);
    expect(back.children.Pipe.attrs.Pipe_Name).toEqual(orig.children.Pipe.attrs.Pipe_Name);
    expect(back.children.Triggers.attrs.Triggers_Enabled).toEqual(orig.children.Triggers.attrs.Triggers_Enabled);
  });

  it('trigger groups and their attributes survive', () => {
    const t = rt.extensions!.TM_OPCUA.children.Triggers.children;
    expect(Object.keys(t).sort()).toEqual(['Chamber Oxygen Level', 'Laser Emission Interlock']);
    expect(val(t['Chamber Oxygen Level'], 'Start_Value')).toBe('700');
    expect(val(t['Laser Emission Interlock'], 'Event')).toBe('SensorEvents');
  });

  it('Trigger_Stop_Ceiling_Layers survives roundtrip (value = 3)', () => {
    const triggers = rt.extensions!.TM_OPCUA.children.Triggers;
    expect(Number(val(triggers, 'Trigger_Stop_Ceiling_Layers'))).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// Roundtrip: synthetic 2-laser fixture
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — synthetic 2-laser roundtrip', () => {
  let rt: MachineConfig;

  beforeAll(async () => {
    rt = await roundtrip(synthetic);
  }, 60_000);

  it('two optical trains survive roundtrip', () => {
    expect(rt.optical_trains.length).toBe(2);
  });

  it('train 1 scan_head_rotation survives roundtrip', () => {
    expect(rt.optical_trains[1].scanner.scan_head_rotation)
      .toBeCloseTo(synthetic.optical_trains[1].scanner.scan_head_rotation!, 5);
  });
});

// ---------------------------------------------------------------------------
// Correction data hash roundtrip
// ---------------------------------------------------------------------------

function correctionHash(data: Float64Array): string {
  const buf = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
  return createHash('sha256').update(buf).digest('hex');
}

describe('MachineConfigWriter — correction data hash roundtrip', () => {
  it('forward correction data hash is preserved across write roundtrip', async () => {
    const originalHash = correctionHash(grid(reference, 0, 'Correction_Data').data);
    const rt = await roundtrip(reference);
    expect(correctionHash(grid(rt, 0, 'Correction_Data').data)).toBe(originalHash);
  }, 60_000);

  it('inverse correction data hash is preserved across write roundtrip', async () => {
    const originalHash = correctionHash(grid(reference, 0, 'Inverse_Correction_Data').data);
    const rt = await roundtrip(reference);
    expect(correctionHash(grid(rt, 0, 'Inverse_Correction_Data').data)).toBe(originalHash);
  }, 60_000);
});
