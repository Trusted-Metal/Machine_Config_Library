import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { MachineConfigWriter } from '../src/index.js';
import { MachineConfigReader } from '../src/index.js';
import type { MachineConfig } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE      = join(__dirname, '../../fixtures/reference_config.h5');
const REFERENCE_OPCUA = join(__dirname, '../../fixtures/reference_config_opcua.h5');
const SYNTHETIC      = join(__dirname, '../../fixtures/synthetic_2laser.h5');

/** Absolute path to a unique temp HDF5 file that does not exist yet. */
function tmpH5(): string {
  return join(tmpdir(), `test_writer_${randomBytes(8).toString('hex')}.h5`);
}

// Module-level fixture configs, parsed once in beforeAll.
let reference: MachineConfig;
let referenceOpcua: MachineConfig;
let synthetic: MachineConfig;

// Paths to temp files created by write tests — deleted in afterAll.
const tmpFiles: string[] = [];

beforeAll(async () => {
  [reference, referenceOpcua, synthetic] = await Promise.all([
    new MachineConfigReader(REFERENCE).parse(),
    new MachineConfigReader(REFERENCE_OPCUA).parse(),
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

  it('configuration_hash survives roundtrip', () => {
    expect(rt.meta.configuration_hash).toBe(reference.meta.configuration_hash);
  });

  it('optical train count survives roundtrip', () => {
    expect(rt.optical_trains.length).toBe(reference.optical_trains.length);
  });

  it('build plate X survives roundtrip', () => {
    expect(rt.machine.build_plate_x).toBeCloseTo(reference.machine.build_plate_x!, 5);
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

  it('ClearBox ip_address survives roundtrip', () => {
    const orig = reference.optical_trains[0].optional_components.clearbox;
    const back = rt.optical_trains[0].optional_components.clearbox;
    expect(back?.ip_address).toBe(orig?.ip_address);
  });

  it('no opcua in non-opcua fixture roundtrip', () => {
    expect(rt.opcua).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Roundtrip: OPC-UA fixture
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — OPC-UA roundtrip', () => {
  let rt: MachineConfig;

  beforeAll(async () => {
    rt = await roundtrip(referenceOpcua);
  }, 60_000);

  it('opcua block is preserved', () => {
    expect(rt.opcua).toBeDefined();
  });

  it('opcua server_url survives roundtrip', () => {
    expect(rt.opcua!.client.server_url).toBe(referenceOpcua.opcua!.client.server_url);
  });

  it('opcua session_timeout survives roundtrip', () => {
    expect(rt.opcua!.client.session_timeout).toBe(referenceOpcua.opcua!.client.session_timeout);
  });

  it('opcua triggers_enabled survives roundtrip', () => {
    expect(rt.opcua!.triggers_enabled).toBe(referenceOpcua.opcua!.triggers_enabled);
  });

  it('opcua trigger names are preserved', () => {
    const origKeys = Object.keys(referenceOpcua.opcua!.triggers).sort();
    const rtKeys   = Object.keys(rt.opcua!.triggers).sort();
    expect(rtKeys).toEqual(origKeys);
  });

  it('opcua trigger signal survives roundtrip', () => {
    const origTriggers = referenceOpcua.opcua!.triggers;
    const firstName = Object.keys(origTriggers)[0];
    expect(rt.opcua!.triggers[firstName]?.signal)
      .toBe(origTriggers[firstName]?.signal);
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
