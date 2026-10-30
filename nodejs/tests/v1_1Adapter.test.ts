/**
 * File_Version 1.1 adapter tests — the real v1.1 adapter (not the mock in
 * `mockV1_1.ts`/`adapterMigration.test.ts`). Mirrors the Rust suite for
 * `docs/migrations/v1_0_to_v1_1.md`'s 5 changes.
 *
 * ClearBox is an opaque extension tree in core, always in the File_Version 1.1 layout
 * (`ClearBox/<train_id>/Power_Characterization/`, shared `Output_Path` and
 * `Software_Trigger_Delay` on the extension root). A File_Version 1.0 file is shimmed into and out
 * of that layout on read and write, so upgrade/downgrade is just parse()/write() using
 * MachineConfigWriter's optional targetVersion parameter.
 */
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import type { MachineConfig } from '../src/models.js';
import { MockConfigBuilder } from '../src/builder.js';
import { Hdf5AdapterV1_0 } from '../src/capabilities/v1_0/hdf5.js';
import { Hdf5AdapterV1_1, Hdf5WriterV1_1 } from '../src/capabilities/v1_1/index.js';
import type { ReadOptions } from '../src/capabilities/v1_1/hdf5.js';
import { forwardPowerCharacterizationPoints } from '../src/powerCharacterization.js';
import { MachineConfigReader, _READERS } from '../src/reader.js';
import { MachineConfigWriter, _WRITERS } from '../src/writer.js';
import { MachineConfigFileV1_1 } from '../src/capabilities/index.js';
import type { MachineConfigFile } from '../src/capabilities/generated.js';
import { canonicalDigest } from '../src/index.js';
import {
  attr,
  clearbox,
  clearboxRoot,
  constants,
  int,
  makePowerCharacterization,
  pointCount,
  powerCharacterization,
  text,
} from './clearboxHelpers.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE_V1_0 = join(__dirname, '../../fixtures/reference_config_opcua_synchronous_sensors.h5');
const REFERENCE_V1_1 = join(__dirname, '../../fixtures/reference_config_v1_1.h5');

function tempH5Path(prefix: string): string {
  return join(tmpdir(), `${prefix}_${randomUUID()}.h5`);
}

function mockV1Config(nLasers = 2): MachineConfig {
  const cfg = new MockConfigBuilder({ nLasers }).build();
  cfg.meta.file_version = '1.0';
  return cfg;
}

/** In-memory v1.1-shaped config: the v1.0 mock with file_version overridden (its ClearBox tree is already in the v1.1 layout). */
function mockV1_1Config(nLasers = 2): MachineConfig {
  const cfg = mockV1Config(nLasers);
  return { ...cfg, meta: { ...cfg.meta, file_version: '1.1' } };
}

async function writeAs(cfg: MachineConfig, path: string, targetVersion: string): Promise<void> {
  await new MachineConfigWriter(cfg, targetVersion).write(path);
}

/** Write `cfg` as `version` and read it back through the public reader. */
async function viaFile(cfg: MachineConfig, version: string): Promise<MachineConfig> {
  const out = tempH5Path(`via_${version}`);
  await writeAs(cfg, out, version);
  return new MachineConfigReader(out).parse();
}

const names = (pc: ReturnType<typeof powerCharacterization>) => constants(pc).map((c) => c.name);
const values = (pc: ReturnType<typeof powerCharacterization>) => constants(pc).map((c) => c.value);
const sorted = (v: number[]) => [...v].sort((a, b) => a - b);

// ---------------------------------------------------------------------------
// read
// ---------------------------------------------------------------------------

describe('Hdf5AdapterV1_1 — read', () => {
  it('parses the natively-authored v1.1 fixture — all 5 changes asserted by value', async () => {
    const cfg = await new Hdf5AdapterV1_1(REFERENCE_V1_1).parse();
    expect(cfg.meta.file_version).toBe('1.1');

    // Change 1: Consolidate — one shared value, on the extension root.
    const root = clearboxRoot(cfg);
    expect(text(root, 'Output_Path')).toBe('/recordings/');
    expect(int(root, 'Software_Trigger_Delay')).toBe(3000);

    cfg.optical_trains.forEach((t, i) => {
      const cb = clearbox(cfg, i);
      // Change 1: Addition
      expect(text(cb, 'Firmware_Version')).toBe('2.4.1');
      // Change 1: Removal
      for (const removed of ['Selected_Camera', 'Custom_Video_Format', 'Video_Output', 'Show_Console', 'Output_Path', 'Software_Trigger_Delay']) {
        expect(cb.attrs).not.toHaveProperty(removed);
      }

      // Change 5: no on-disk source in v1.1, always null
      expect(t.scanner.x_axis?.tuning_parameters).toBeNull();
      expect(t.scanner.x_axis?.tuning_type).toBeNull();
      expect(t.scanner.y_axis?.tuning_parameters).toBeNull();
      expect(t.scanner.y_axis?.tuning_type).toBeNull();
    });

    // Change 2: OPCUA relocated, contents unaffected
    expect(cfg.extensions?.TM_OPCUA).toBeDefined();
    expect(cfg).not.toHaveProperty('opcua');

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL
    const pc0 = powerCharacterization(cfg, 0);
    expect(text(pc0, 'Algorithm_Type')).toBe('LINEAR');
    expect(text(pc0, 'Algorithm_Equation')).toBe('W = a*V + b');
    expect(text(pc0, 'Input_Type')).toBe('0-10 V');
    expect(text(pc0, 'Units_Derived_Quantity')).toBe('Watts');
    expect(names(pc0)).toEqual(['b', 'a']);
    expect(pointCount(pc0)).toBe(3);

    const pc1 = powerCharacterization(cfg, 1);
    expect(text(pc1, 'Algorithm_Type')).toBe('POLYNOMIAL');
    expect(text(pc1, 'Algorithm_Equation')).toBe('W = c0 + c1*V + c2*V^2');
    expect(names(pc1)).toEqual(['c0', 'c1', 'c2']);

    // Change 4: LightSource Power_Characterization — inverse data availability
    const lspc0 = cfg.optical_trains[0].light_source.power_characterization!;
    expect(lspc0.algorithm_type).toBe('LINEAR');
    expect(lspc0.input_type).toBe('Volts');
    expect(lspc0.characterization_points.length).toBe(5);
    expect(lspc0.derivation_equation_constants.length).toBeGreaterThan(0);
  });

  it('toJson() includes the ClearBox extension and the light source power_characterization', async () => {
    const json = await new Hdf5AdapterV1_1(REFERENCE_V1_1).toJson();
    expect(json).toContain('"Firmware_Version"');
    expect(json).toContain('"power_characterization"');
  });
});

// ---------------------------------------------------------------------------
// roundtrip
// ---------------------------------------------------------------------------

describe('Hdf5AdapterV1_1 / Hdf5WriterV1_1 — roundtrip', () => {
  it('write -> read -> write -> read, no drift; ClearBox/LightSource cross-wiring guard', async () => {
    const cfg = mockV1_1Config(1);
    clearbox(cfg, 0).children['Power_Characterization'] = makePowerCharacterization('LINEAR', [['b', 1.0], ['a', 2.0]]);
    const train = cfg.optical_trains[0];
    cfg.optical_trains = [
      {
        ...train,
        light_source: {
          ...train.light_source,
          power_characterization: {
            algorithm_type: 'POLYNOMIAL',
            algorithm_equation: null,
            input_type: null,
            units_derived_quantity: null,
            derivation_equation_constants: [],
            characterization_points: [{ input_value: 9.0, output_value: 99.0 }],
          },
        },
      },
    ];

    const p1 = tempH5Path('v1_1_a');
    await new Hdf5WriterV1_1(cfg).write(p1);
    const mid = await new Hdf5AdapterV1_1(p1).parse();
    const p2 = tempH5Path('v1_1_b');
    await new Hdf5WriterV1_1(mid).write(p2);
    const result = await new Hdf5AdapterV1_1(p2).parse();

    const cbPc = powerCharacterization(result, 0);
    const lsPc = result.optical_trains[0].light_source.power_characterization!;
    expect(text(cbPc, 'Algorithm_Type')).toBe('LINEAR');
    expect(names(cbPc)).toEqual(['b', 'a']);
    expect(pointCount(cbPc)).toBe(0);
    expect(lsPc.algorithm_type).toBe('POLYNOMIAL');
    expect(lsPc.derivation_equation_constants).toEqual([]);
    expect(lsPc.characterization_points[0].input_value).toBe(9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    expect(text(cbPc, 'Algorithm_Type')).not.toBe(lsPc.algorithm_type);
    // The whole ClearBox tree, grids included, is bit-stable across the double round trip.
    expect(canonicalDigest(result.extensions!.ClearBox)).toBe(canonicalDigest(cfg.extensions!.ClearBox));
  });
});

// ---------------------------------------------------------------------------
// v1.0 fixture written as v1.1
// ---------------------------------------------------------------------------

describe('v1.0 fixture written as v1.1', () => {
  it('asserts the manifest\'s actual rules', async () => {
    const source = await new Hdf5AdapterV1_0(REFERENCE_V1_0).parse();
    const out = tempH5Path('v1_1_from_v1_0');
    await writeAs(source, out, '1.1');
    const migrated = await new Hdf5AdapterV1_1(out).parse();
    expect(migrated.meta.file_version).toBe('1.1');

    // The shimmed tree is already in the v1.1 layout, so it survives unchanged.
    expect(canonicalDigest(migrated.extensions!.ClearBox)).toBe(canonicalDigest(source.extensions!.ClearBox));
    expect(clearboxRoot(migrated).attrs).toHaveProperty('Output_Path');
    expect(clearboxRoot(migrated).attrs).toHaveProperty('Software_Trigger_Delay');

    migrated.optical_trains.forEach((t, i) => {
      const cb = clearbox(migrated, i);
      // Addition: no v1.0 source.
      expect(cb.attrs).not.toHaveProperty('Firmware_Version');
      // v1.0-only attributes ride along as extra per-train attributes.
      expect(cb.attrs).toHaveProperty('Selected_Camera');
      // Change 3: derived ClearBox data has real constants, zero points.
      const pc = powerCharacterization(migrated, i);
      expect(pointCount(pc)).toBe(0);
      expect(constants(pc).length).toBeGreaterThan(0);
      expect(text(pc, 'Algorithm_Type')).toBe(text(powerCharacterization(source, i), 'Algorithm_Type'));

      // Change 4: derived Light_Source data is the inverse — zero constants, real points.
      const ls = t.light_source;
      expect(ls.power_characterization?.derivation_equation_constants).toEqual([]);
      expect(ls.power_characterization?.characterization_points.length).toBeGreaterThan(0);
    });
  });

  it('the first train wins for the shared Output_Path / Software_Trigger_Delay in a v1.0 file', async () => {
    // File_Version 1.0 stores them per train; the extension tree holds one value each.
    const cfg = mockV1Config(2);
    clearbox(cfg, 1).attrs['Output_Path'] = { dtype: 'utf8', value: '/other/' };
    clearbox(cfg, 1).attrs['Software_Trigger_Delay'] = { dtype: 'i64', value: 9999n };
    const back = await viaFile(cfg, '1.0');
    const root = clearboxRoot(back);
    expect(text(root, 'Output_Path')).toBe('/recordings/');
    expect(int(root, 'Software_Trigger_Delay')).toBe(3000);
    expect(clearbox(back, 1).attrs).not.toHaveProperty('Output_Path');
  });

  it('unrecognized Algorithm_Type on ClearBox is best-effort, never raises (Phase 2)', async () => {
    const cfg = mockV1_1Config(1);
    clearbox(cfg, 0).children['Power_Characterization'] = makePowerCharacterization('EXPONENTIAL', [
      ['0', 1.5],
      ['1', 2.5],
      ['2', 3.5],
    ]);
    const v1_0 = await viaFile(cfg, '1.0');
    const pc = powerCharacterization(v1_0, 0);
    expect(text(pc, 'Algorithm_Type')).toBe('EXPONENTIAL');
    expect(text(pc, 'Algorithm_Equation')).toBeNull();
    expect(names(pc)).toEqual(['0', '1', '2']);
    expect(values(pc)).toEqual([1.5, 2.5, 3.5]);

    const back = await viaFile(v1_0, '1.1');
    expect(text(powerCharacterization(back, 0), 'Algorithm_Type')).toBe('EXPONENTIAL');
    expect(values(powerCharacterization(back, 0))).toEqual([1.5, 2.5, 3.5]);
  });

  it('unrecognized Algorithm_Type on LightSource is best-effort, never raises (Phase 2)', async () => {
    const cfg = mockV1Config(1);
    const t0 = cfg.optical_trains[0];
    cfg.optical_trains = [
      { ...t0, light_source: { ...t0.light_source, power_characterization: forwardPowerCharacterizationPoints('QUADRATIC', '1,2,3,4') } },
    ];
    const result = await viaFile(cfg, '1.1');
    const pc = result.optical_trains[0].light_source.power_characterization!;
    expect(pc.algorithm_type).toBe('QUADRATIC');
    expect(pc.algorithm_equation).toBeNull();
    expect(pc.characterization_points.length).toBe(2);
  });

  it('a ClearBox train with no matching optical train is rejected for v1.0', async () => {
    const cfg = mockV1_1Config(1);
    cfg.extensions!.ClearBox.children['Optical_Train_07'] = structuredClone(clearbox(cfg, 0));
    await expect(writeAs(cfg, tempH5Path('no_matching_train'), '1.0')).rejects.toThrow(/Optical_Train_07/);
  });
});

// ---------------------------------------------------------------------------
// v1.1 fixture written as v1.0
// ---------------------------------------------------------------------------

describe('v1.1 fixture written as v1.0', () => {
  it('preserves surviving fields and re-derives Volts_To_Watts_*/Watts_To_Volts_* correctly', async () => {
    const source = await new Hdf5AdapterV1_1(REFERENCE_V1_1).parse();
    const out = tempH5Path('v1_0_from_v1_1');
    await writeAs(source, out, '1.0');
    const back = await new Hdf5AdapterV1_0(out).parse();
    expect(back.meta.file_version).toBe('1.0');

    const srcPc0 = powerCharacterization(source, 0);
    const pc0 = powerCharacterization(back, 0);
    expect(text(pc0, 'Algorithm_Type')).toBe(text(srcPc0, 'Algorithm_Type'));
    expect(sorted(values(pc0))).toEqual(sorted(values(srcPc0)));
    const [srcRoot, root] = [clearboxRoot(source), clearboxRoot(back)];
    expect(root.attrs['Output_Path']).toEqual(srcRoot.attrs['Output_Path']);
    expect(Number(attr(root, 'Software_Trigger_Delay'))).toBe(Number(attr(srcRoot, 'Software_Trigger_Delay')));

    const srcLsPc0 = source.optical_trains[0].light_source.power_characterization!;
    const lsPc0 = back.optical_trains[0].light_source.power_characterization!;
    expect(lsPc0.algorithm_type).toBe(srcLsPc0.algorithm_type);
    const flat = (p: typeof srcLsPc0) => p.characterization_points.flatMap((x) => [x.input_value, x.output_value]);
    expect(flat(lsPc0)).toEqual(flat(srcLsPc0));

    // v1.1-only fields: lost on the way down, not restored.
    expect(clearbox(back, 0).attrs).not.toHaveProperty('Firmware_Version');
    expect(clearbox(back, 0).attrs).not.toHaveProperty('Selected_Camera');
  });

  it('reorders Derivation_Equation_Constants by name before joining as CSV', async () => {
    const cfg = mockV1_1Config(1);
    clearbox(cfg, 0).children['Power_Characterization'] = makePowerCharacterization('LINEAR', [['a', 2], ['b', 1]]);
    const back = await viaFile(cfg, '1.0');
    // Written CSV is "1,2" (b before a); the v1.0 reader forward-derives that back into named
    // constants positionally (b, then a) for LINEAR.
    expect(constants(powerCharacterization(back, 0))).toEqual([
      { name: 'b', value: 1 },
      { name: 'a', value: 2 },
    ]);
  });

  it('writes a blank Volts_To_Watts_Params when Derivation_Equation_Constants is empty', async () => {
    const cfg = mockV1_1Config(1);
    clearbox(cfg, 0).children['Power_Characterization'] = makePowerCharacterization('LINEAR', []);
    const back = await viaFile(cfg, '1.0');
    // Backward derivation produces "" (blank, not an error); forward re-derivation then yields zero constants.
    expect(constants(powerCharacterization(back, 0))).toEqual([]);
  });

  it('writes a blank Watts_To_Volts_Params when Characterization_Points is empty', async () => {
    const cfg = mockV1_1Config(1);
    const t0 = cfg.optical_trains[0];
    cfg.optical_trains = [
      { ...t0, light_source: { ...t0.light_source, power_characterization: { ...t0.light_source.power_characterization!, characterization_points: [] } } },
    ];
    const back = await viaFile(cfg, '1.0');
    expect(back.optical_trains[0].light_source.power_characterization?.characterization_points).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// v1 unaffected
// ---------------------------------------------------------------------------

describe('v1.0 path unaffected by v1.1 existing', () => {
  it('the v1.0 read path surfaces the flat Volts_To_Watts fields as Power_Characterization', async () => {
    const config = await new MachineConfigReader(REFERENCE_V1_0).parse();
    expect(config.meta.file_version).toBe('1.0');
    expect(config.optical_trains.length).toBeGreaterThan(0);
    expect(text(powerCharacterization(config, 0), 'Algorithm_Type')).toBe('LINEAR');
    expect(clearbox(config, 0).attrs).not.toHaveProperty('Volts_To_Watts_Algorithm');
  });
});

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

describe('round-trip acceptance criterion', () => {
  it('v1.0 -> v1.1 -> v1.0 reproduces the original exactly', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_0).parse();
    const mid = await viaFile(source, '1.1');
    const back = await viaFile(mid, '1.0');

    expect(back.meta.file_version).toBe('1.0');
    expect(canonicalDigest(back.extensions!.ClearBox)).toBe(canonicalDigest(source.extensions!.ClearBox));
    back.optical_trains.forEach((t, i) => {
      const srcLsPc = source.optical_trains[i].light_source.power_characterization!;
      const lsPc = t.light_source.power_characterization!;
      expect(lsPc.algorithm_type).toBe(srcLsPc.algorithm_type);
      const flat = (p: typeof srcLsPc) => p.characterization_points.flatMap((x) => [x.input_value, x.output_value]);
      expect(flat(lsPc)).toEqual(flat(srcLsPc));
    });
  });

  it('v1.1 -> v1.0 -> v1.1: fields v1.0 can hold survive, the rest comes back blank', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_1).parse();
    const mid = await viaFile(source, '1.0');
    const back = await viaFile(mid, '1.1');

    expect(back.meta.file_version).toBe('1.1');
    back.optical_trains.forEach((_, i) => {
      const srcPc = powerCharacterization(source, i);
      const pc = powerCharacterization(back, i);
      expect(text(pc, 'Algorithm_Type')).toBe(text(srcPc, 'Algorithm_Type'));
      expect(sorted(values(pc))).toEqual(sorted(values(srcPc)));
      // Expected loss: v1.1-only fields have no v1.0 round-trip path.
      expect(clearbox(back, i).attrs).not.toHaveProperty('Firmware_Version');
      expect(text(pc, 'Input_Type')).toBeNull();
      expect(text(pc, 'Units_Derived_Quantity')).toBeNull();
      expect(pointCount(pc)).toBe(0);
    });
  });
});

// ---------------------------------------------------------------------------
// Writer targetVersion parameter
// ---------------------------------------------------------------------------

describe('MachineConfigWriter targetVersion', () => {
  it('overrides meta.file_version without mutating the input', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_0).parse();
    expect(source.meta.file_version).toBe('1.0');
    const result = await viaFile(source, '1.1');
    expect(result.meta.file_version).toBe('1.1');
    expect(source.meta.file_version).toBe('1.0');

    const v1_1Source = await new MachineConfigReader(REFERENCE_V1_1).parse();
    expect(v1_1Source.meta.file_version).toBe('1.1');
    const result2 = await viaFile(v1_1Source, '1.0');
    expect(result2.meta.file_version).toBe('1.0');
    expect(v1_1Source.meta.file_version).toBe('1.1');
  });

  it('defaults to meta.file_version when targetVersion is omitted', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_0).parse();
    const out = tempH5Path('target_version_default');
    await new MachineConfigWriter(source).write(out); // no targetVersion
    const result = await new MachineConfigReader(out).parse();
    expect(result.meta.file_version).toBe('1.0');
  });
});

// ---------------------------------------------------------------------------
// Dispatcher-level tests — full public API via the real "1.1" registry entry
// ---------------------------------------------------------------------------

describe('dispatcher — real "1.1" registry entry', () => {
  it('"1.0" and "1.1" are both registered on the real reader/writer dispatch tables', () => {
    expect(_READERS['1.0']).toBeDefined();
    expect(_READERS['1.1']).toBeDefined();
    expect(_WRITERS['1.0']).toBeDefined();
    expect(_WRITERS['1.1']).toBeDefined();
  });

  it('routes a real v1.0 fixture written as v1.1 through the public API', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_0).parse();
    const result = await viaFile(source, '1.1');
    expect(result.meta.file_version).toBe('1.1');
    expect(text(clearboxRoot(result), 'Output_Path')).toBe('/recordings/');
    expect(clearbox(result, 0).children).toHaveProperty('Power_Characterization');
  });

  it('routes the natively-authored v1.1 fixture written as v1.0 through the public API', async () => {
    const v1_1Cfg = await new MachineConfigReader(REFERENCE_V1_1).parse();
    const result = await viaFile(v1_1Cfg, '1.0');
    expect(result.meta.file_version).toBe('1.0');
    expect(text(powerCharacterization(result, 0), 'Algorithm_Type')).toBe('LINEAR');
  });
});

// ---------------------------------------------------------------------------
// Adapters satisfy the expected structural shape / interface
// ---------------------------------------------------------------------------

describe('adapters satisfy the expected interfaces', () => {
  it('Hdf5AdapterV1_1 / Hdf5WriterV1_1 satisfy the reader/writer dispatch shapes', () => {
    const readerCtor: new (path: string) => { parse: (o?: ReadOptions) => Promise<MachineConfig> } =
      Hdf5AdapterV1_1;
    const writerCtor: new (config: MachineConfig) => { write: (path: string) => Promise<void> } = Hdf5WriterV1_1;
    expect(readerCtor).toBe(Hdf5AdapterV1_1);
    expect(writerCtor).toBe(Hdf5WriterV1_1);
  });

  it('MachineConfigFileV1_1 satisfies the MachineConfigFile capability interface', () => {
    const created = MachineConfigFileV1_1.create('1.1');
    expect(created.ok).toBe(true);
    if (!created.ok) return;
    const facade: MachineConfigFile = created.value;
    expect(facade.fileVersion()).toBe('1.1');
    expect(facade.opticalTrains().length).toBeGreaterThan(0);
    facade.close();
  });
});
