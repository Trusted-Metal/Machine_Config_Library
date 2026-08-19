import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes, createHash } from 'node:crypto';
import { MachineConfigWriter } from '../src/index.js';
import { MachineConfigReader } from '../src/index.js';
import { UnsupportedFileVersion } from '../src/capabilities/index.js';
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

  it('"Chamber Oxygen Level" trigger subsystem survives roundtrip', () => {
    expect(rt.opcua!.triggers['Chamber Oxygen Level']?.subsystem)
      .toBe(referenceOpcua.opcua!.triggers['Chamber Oxygen Level']?.subsystem);
  });

  it('"Chamber Oxygen Level" trigger rule_enabled survives roundtrip', () => {
    expect(rt.opcua!.triggers['Chamber Oxygen Level']?.rule_enabled)
      .toBe(referenceOpcua.opcua!.triggers['Chamber Oxygen Level']?.rule_enabled);
  });

  it('"Chamber Oxygen Level" trigger start_value and stop_value survive roundtrip', () => {
    const orig = referenceOpcua.opcua!.triggers['Chamber Oxygen Level']!;
    const rtT  = rt.opcua!.triggers['Chamber Oxygen Level']!;
    expect(rtT.start_value).toBe(orig.start_value);
    expect(rtT.stop_value).toBe(orig.stop_value);
  });

  // Newly-promoted fields (OPCUA_FIELD_PROMOTION_PLAN.md Phase 1) — the
  // 22 promoted fields survive a real write→read cycle, not just parsing.
  it('client promoted fields survive roundtrip', () => {
    const orig = referenceOpcua.opcua!.client;
    const rtC  = rt.opcua!.client;
    expect(rtC.keep_alive_count).toBe(orig.keep_alive_count);
    expect(rtC.lifetime_count).toBe(orig.lifetime_count);
    expect(rtC.machine_profile).toBe(orig.machine_profile);
    expect(rtC.queue_policy).toBe(orig.queue_policy);
    expect(rtC.queue_size_data_change).toBe(orig.queue_size_data_change);
    expect(rtC.queue_size_events).toBe(orig.queue_size_events);
    expect(rtC.reconnect_interval).toBe(orig.reconnect_interval);
    expect(rtC.root_node).toBe(orig.root_node);
    expect(rtC.sync_loop_interval_initial).toBe(orig.sync_loop_interval_initial);
    expect(rtC.sync_loop_interval_settled).toBe(orig.sync_loop_interval_settled);
  });

  it('pipe promoted fields survive roundtrip', () => {
    const orig = referenceOpcua.opcua!.pipe;
    const rtP  = rt.opcua!.pipe;
    expect(rtP.configure_client).toBe(orig.configure_client);
    expect(rtP.inbound_rate_limit).toBe(orig.inbound_rate_limit);
    expect(rtP.max_inbound_message_size).toBe(orig.max_inbound_message_size);
    expect(rtP.min_integrity_level).toBe(orig.min_integrity_level);
    expect(rtP.pipe_name).toBe(orig.pipe_name);
    expect(rtP.user_access_level).toBe(orig.user_access_level);
  });

  it('"Laser Emission Interlock" trigger promoted fields survive roundtrip', () => {
    const orig = referenceOpcua.opcua!.triggers['Laser Emission Interlock']!;
    const rtT  = rt.opcua!.triggers['Laser Emission Interlock']!;
    expect(rtT.case_sensitivity).toBe(orig.case_sensitivity);
    expect(rtT.component).toBe(orig.component);
    expect(rtT.cooldown_period).toBe(orig.cooldown_period);
    expect(rtT.event).toBe(orig.event);
    expect(rtT.max_fires_per_job).toBe(orig.max_fires_per_job);
    expect(rtT.trigger_label).toBe(orig.trigger_label);
  });

  it('trigger_stop_ceiling_layers survives roundtrip (value = 3)', () => {
    expect(referenceOpcua.opcua!.trigger_stop_ceiling_layers).toBe(3);
    expect(rt.opcua!.trigger_stop_ceiling_layers).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// trigger_stop_ceiling_layers — dedicated null/non-null roundtrip
// ---------------------------------------------------------------------------

describe('MachineConfigWriter — trigger_stop_ceiling_layers roundtrip', () => {
  it('round-trips a real value (3)', async () => {
    const rt = await roundtrip(referenceOpcua);
    expect(rt.opcua!.trigger_stop_ceiling_layers).toBe(3);
  });

  it('round-trips null when cleared', async () => {
    const cleared: MachineConfig = {
      ...referenceOpcua,
      opcua: { ...referenceOpcua.opcua!, trigger_stop_ceiling_layers: null },
    };
    const rt = await roundtrip(cleared);
    expect(rt.opcua!.trigger_stop_ceiling_layers).toBeNull();
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
    const srcReader = new MachineConfigReader(REFERENCE);
    const originalFwd = await srcReader.getCorrectionData(0);
    const originalHash = correctionHash(originalFwd.data);

    const src = await srcReader.parse({ includeBinary: true });
    const out = tmpH5();
    tmpFiles.push(out);
    await new MachineConfigWriter(src).write(out);
    const rtFwd = await new MachineConfigReader(out).getCorrectionData(0);
    expect(correctionHash(rtFwd.data)).toBe(originalHash);
  }, 60_000);

  it('inverse correction data hash is preserved across write roundtrip', async () => {
    const srcReader = new MachineConfigReader(REFERENCE);
    const originalInv = await srcReader.getInverseCorrectionData(0);
    const originalHash = correctionHash(originalInv.data);

    const src = await srcReader.parse({ includeBinary: true });
    const out = tmpH5();
    tmpFiles.push(out);
    await new MachineConfigWriter(src).write(out);
    const rtInv = await new MachineConfigReader(out).getInverseCorrectionData(0);
    expect(correctionHash(rtInv.data)).toBe(originalHash);
  }, 60_000);
});
