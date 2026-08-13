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
import type { MachineConfig } from '../src/models.js';

// ---------------------------------------------------------------------------
// Fixture paths (absolute — resolved relative to this test file)
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const SYNTHETIC       = join(__dirname, '../../fixtures/synthetic_2laser.h5');
const REFERENCE       = join(__dirname, '../../fixtures/reference_config.h5');
const REFERENCE_OPCUA = join(__dirname, '../../fixtures/reference_config_opcua.h5');

// ---------------------------------------------------------------------------
// Shared parsed configs — each fixture opened once for the whole suite
// ---------------------------------------------------------------------------

let synthetic: MachineConfig;
let reference: MachineConfig;
let referenceOpcua: MachineConfig;

beforeAll(async () => {
  [synthetic, reference, referenceOpcua] = await Promise.all([
    new MachineConfigReader(SYNTHETIC).parse(),
    new MachineConfigReader(REFERENCE).parse(),
    new MachineConfigReader(REFERENCE_OPCUA).parse(),
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

  it('optional_components object is always present (synthetic)', () => {
    for (const train of synthetic.optical_trains) {
      expect(train.optional_components).toBeDefined();
      expect(train.optional_components).not.toBeNull();
    }
  });

  it('optional_components object is always present (reference)', () => {
    for (const train of reference.optical_trains) {
      expect(train.optional_components).toBeDefined();
      expect(train.optional_components).not.toBeNull();
    }
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
    expect(reference.optical_trains[0].optional_components.clearbox).not.toBeNull();
  });

  it('clearbox.ip_address is non-empty (reference)', () => {
    expect(reference.optical_trains[0].optional_components.clearbox?.ip_address).toBeTruthy();
  });

  it('clearbox.data_port is an integer or null (reference)', () => {
    const dp = reference.optical_trains[0].optional_components.clearbox?.data_port;
    expect(dp === null || Number.isInteger(dp)).toBe(true);
  });

  it('correction_data is absent by default (parse without includeBinary)', () => {
    const cd = reference.optical_trains[0].optional_components.clearbox?.correction_data;
    expect(cd).toBeUndefined();
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

describe('MachineConfigReader — correction data (includeBinary: true)', () => {
  let refBinary: MachineConfig;

  beforeAll(async () => {
    refBinary = await new MachineConfigReader(REFERENCE).parse({ includeBinary: true });
  }, 60_000);

  it('correction_data is present when includeBinary=true', () => {
    const cd = refBinary.optical_trains[0].optional_components.clearbox?.correction_data;
    expect(cd).toBeDefined();
    expect(cd).not.toBeNull();
  });

  it('correction_data shape is [257][257][2]', () => {
    const cd = refBinary.optical_trains[0].optional_components.clearbox!.correction_data!;
    expect(cd).toHaveLength(257);
    expect(cd[0]).toHaveLength(257);
    expect(cd[0][0]).toHaveLength(2);
  });

  it('correction_data contains null values (border NaN cells)', () => {
    const cd = refBinary.optical_trains[0].optional_components.clearbox!.correction_data!;
    const hasNull = cd.some(row => row.some(cell => cell.some(v => v === null)));
    expect(hasNull).toBe(true);
  });

  it('inverse_correction_data shape is [257][257][2]', () => {
    const icd = refBinary.optical_trains[0].optional_components.clearbox!.inverse_correction_data!;
    expect(icd).toBeDefined();
    expect(icd).toHaveLength(257);
    expect(icd[0]).toHaveLength(257);
    expect(icd[0][0]).toHaveLength(2);
  });

  it('correction_data is absent when parsed without includeBinary', () => {
    const cd = reference.optical_trains[0].optional_components.clearbox?.correction_data;
    expect(cd).toBeUndefined();
  });
});

// ===========================================================================
// Raw correction data accessors (getCorrectionData / getInverseCorrectionData)
// — bypass the null-converted JSON representation; feed the correction-hash CLI.
// ===========================================================================

describe('MachineConfigReader — getCorrectionData / getInverseCorrectionData', () => {
  const reader = new MachineConfigReader(REFERENCE);

  /** Byte view of a Float64Array's own data (respects byteOffset/byteLength). */
  const bytesOf = (a: Float64Array) => Buffer.from(a.buffer, a.byteOffset, a.byteLength);

  // Read all three large datasets once in parallel so individual tests are
  // synchronous and don't accumulate per-test h5wasm open/read/close overhead.
  let cd0: Awaited<ReturnType<typeof reader.getCorrectionData>>;
  let icd0: Awaited<ReturnType<typeof reader.getCorrectionData>>;
  let cd1: Awaited<ReturnType<typeof reader.getCorrectionData>>;

  beforeAll(async () => {
    [cd0, icd0, cd1] = await Promise.all([
      reader.getCorrectionData(0),
      reader.getInverseCorrectionData(0),
      reader.getCorrectionData(1),
    ]);
  }, 15_000);

  it('getCorrectionData returns a flat Float64Array of shape [257, 257, 2]', () => {
    expect(cd0.data).toBeInstanceOf(Float64Array);
    expect(cd0.shape).toEqual([257, 257, 2]);
    expect(cd0.data.length).toBe(257 * 257 * 2);
  });

  it('getCorrectionData preserves NaN (border cells are not converted to null/0)', () => {
    let sawNaN = false;
    for (let i = 0; i < cd0.data.length; i++) {
      if (Number.isNaN(cd0.data[i])) { sawNaN = true; break; }
    }
    expect(sawNaN).toBe(true);
  });

  it('getInverseCorrectionData returns a flat Float64Array of shape [257, 257, 2]', () => {
    expect(icd0.data).toBeInstanceOf(Float64Array);
    expect(icd0.shape).toEqual([257, 257, 2]);
  });

  it('forward and inverse correction grids are not identical', () => {
    expect(bytesOf(cd0.data)).not.toEqual(bytesOf(icd0.data));
  });

  it('train 1 correction grid differs from train 0', () => {
    expect(bytesOf(cd0.data)).not.toEqual(bytesOf(cd1.data));
  });

  it('reading the same train twice is deterministic (byte-identical)', async () => {
    // One additional independent read — verifies the cached result is reproducible.
    const again = await reader.getCorrectionData(0);
    expect(bytesOf(again.data)).toEqual(bytesOf(cd0.data));
  }, 10_000);
});

// ===========================================================================
// OPCUA
// ===========================================================================

describe('MachineConfigReader — OPCUA', () => {
  it('opcua is undefined for the reference fixture (no OPCUA group)', () => {
    expect(reference.opcua).toBeUndefined();
  });

  it('opcua is defined for the reference_opcua fixture', () => {
    expect(referenceOpcua.opcua).toBeDefined();
  });

  it('client.bfs_max_depth is 16 (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.bfs_max_depth).toBe(16);
  });

  it('client.publish_interval is 250 (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.publish_interval).toBe(250);
  });

  it('client.sampling_interval is 250 (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.sampling_interval).toBe(250);
  });

  it('client.session_timeout is 60000 (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.session_timeout).toBe(60000);
  });

  it('client.auth_mode is "UsernamePassword" (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.auth_mode).toBe('UsernamePassword');
  });

  it('client.server_url is non-empty (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.client.server_url).toBeTruthy();
  });

  it('pipe.pipe_enabled is true (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.pipe.pipe_enabled).toBe(true);
  });

  it('pipe.buffer_size is 65536 (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.pipe.buffer_size).toBe(65536);
  });

  it('triggers_enabled is true (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.triggers_enabled).toBe(true);
  });

  it('triggers has "Laser Emission Interlock" key (reference_opcua)', () => {
    expect(referenceOpcua.opcua!.triggers).toHaveProperty('Laser Emission Interlock');
  });

  it('"Laser Emission Interlock" trigger has correct id, signal, and subsystem (reference_opcua)', () => {
    const t = referenceOpcua.opcua!.triggers['Laser Emission Interlock'];
    expect(t.id).toBe('trigger_1');
    expect(t.signal).toBe('yellow_light');
    expect(t.subsystem).toBe('Chamber');
    expect(t.rule_enabled).toBe(true);
  });

  it('"Chamber Oxygen Level" trigger has correct start_value and stop_value (reference_opcua)', () => {
    const t = referenceOpcua.opcua!.triggers['Chamber Oxygen Level'];
    expect(t.id).toBe('trigger_2');
    expect(t.signal).toBe('oxygen_level');
    expect(t.start_value).toBe('700');
    expect(t.stop_value).toBe('1000');
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

  it('opcua key is absent from reference fixture JSON', async () => {
    const json = await new MachineConfigReader(REFERENCE).toJson();
    expect(JSON.parse(json)).not.toHaveProperty('opcua');
  });

  it('opcua key is present in reference_opcua fixture JSON', async () => {
    const json = await new MachineConfigReader(REFERENCE_OPCUA).toJson();
    expect(JSON.parse(json)).toHaveProperty('opcua');
  });

  it('correction_data is absent from JSON by default', async () => {
    const json = await new MachineConfigReader(REFERENCE).toJson();
    const parsed = JSON.parse(json);
    const cb = parsed.optical_trains?.[0]?.optional_components?.clearbox;
    expect(cb?.correction_data).toBeUndefined();
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
    expect(validate(synthetic)).toEqual([]);
  });

  it('reference fixture output passes schema validation', () => {
    expect(validate(reference)).toEqual([]);
  });

  it('reference_opcua fixture output passes schema validation', () => {
    expect(validate(referenceOpcua)).toEqual([]);
  });
});
