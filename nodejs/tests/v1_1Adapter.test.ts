/**
 * File_Version 1.1 adapter tests — the real v1.1 adapter (not the mock in
 * `mockV1_1.ts`/`adapterMigration.test.ts`). Mirrors the other 4 languages'
 * rewritten test suites for `docs/migrations/v1_0_to_v1_1.md`'s 5 changes.
 *
 * Phase 2 clean-up (see the migration implementation plan) removed
 * migrateV1ToV1_1/migrateV1_1ToV1 — upgrade/downgrade is now just
 * parse()/write(), using MachineConfigWriter's optional targetVersion
 * parameter. Every test below that used to call a migrate function directly
 * now goes through a real HDF5 write+read instead — a genuine
 * strengthening, since it now exercises the same code path a real caller
 * uses.
 *
 * Tests
 * -----
 *   read                                          natively-authored v1.1 fixture -> StableModel, all 5 changes asserted
 *   roundtrip                                     write -> read -> write -> read, cross-wiring guard
 *   forward: v1.0 fixture written as v1.1         manifest rules
 *   Output_Path / Software_Trigger_Delay disagreement raises
 *   unrecognized Algorithm_Type (ClearBox/LightSource) — best-effort, never raises (Phase 2)
 *   backward: v1.1 fixture written as v1.0        surviving fields preserved
 *   backward reorders constants by name / missing-data writes blank
 *   v1 unaffected                                 v1.0 path undisturbed
 *   v1.0 write never invokes the fallback when native data is present
 *   roundtrip v1.0 -> v1.1 -> v1.0 / v1.1 -> v1.0 -> v1.1 (the acceptance criterion)
 *   writer targetVersion overrides meta / defaults to meta.file_version
 *   dispatcher forward / backward                 full public API, real registry
 *   adapters satisfy interface
 *   toJson() includes the new v1.1 fields
 */
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import type {
  CalibrationPoint,
  ClearBox,
  EquationConstant,
  LightSource,
  MachineConfig,
  OptionalComponents,
} from '../src/models.js';
import { MockConfigBuilder } from '../src/builder.js';
import { Hdf5AdapterV1_0 } from '../src/capabilities/v1_0/hdf5.js';
import { Hdf5AdapterV1_1, Hdf5WriterV1_1 } from '../src/capabilities/v1_1/index.js';
import type { ReadOptions } from '../src/capabilities/v1_1/hdf5.js';
import {
  forwardPowerCharacterizationCoefficients,
  forwardPowerCharacterizationPoints,
} from '../src/powerCharacterization.js';
import { MachineConfigReader, _READERS } from '../src/reader.js';
import { MachineConfigWriter, _WRITERS } from '../src/writer.js';
import { MachineConfigFileV1_1 } from '../src/capabilities/index.js';
import type { MachineConfigFile } from '../src/capabilities/generated.js';

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

/**
 * In-memory v1.1-shaped MachineConfig for tests that need one without
 * touching disk — MockConfigBuilder already builds power_characterization
 * natively, so this is just the v1.0 mock with file_version overridden.
 */
function mockV1_1Config(nLasers = 2): MachineConfig {
  const cfg = mockV1Config(nLasers);
  return { ...cfg, meta: { ...cfg.meta, file_version: '1.1' } };
}

async function writeAs(cfg: MachineConfig, path: string, targetVersion: string): Promise<void> {
  await new MachineConfigWriter(cfg, targetVersion).write(path);
}

// ---------------------------------------------------------------------------
// read
// ---------------------------------------------------------------------------

describe('Hdf5AdapterV1_1 — read', () => {
  it('parses the natively-authored v1.1 fixture — all 5 changes asserted by value', async () => {
    const cfg = await new Hdf5AdapterV1_1(REFERENCE_V1_1).parse();
    expect(cfg.meta.file_version).toBe('1.1');

    for (const t of cfg.optical_trains) {
      const cb = t.optional_components.clearbox as ClearBox;
      // Change 1: Consolidate — same shared value on every train
      expect(cb.output_path).toBe('/recordings/');
      expect(cb.software_trigger_delay).toBe(3000);
      // Change 1: Addition
      expect(cb.firmware_version).toBe('2.4.1');
      // Change 1: Removal
      expect(cb.selected_camera).toBeNull();
      expect(cb.custom_video_format).toBeNull();
      expect(cb.video_output).toBeNull();
      expect(cb.show_console).toBeNull();
      expect(cb.correction_grid_domain_shape).toBeNull();
      expect(cb.inverse_grid_domain_shape).toBeNull();

      // Change 5: no on-disk source in v1.1, always null
      expect(t.scanner.x_axis?.tuning_parameters).toBeNull();
      expect(t.scanner.x_axis?.tuning_type).toBeNull();
      expect(t.scanner.y_axis?.tuning_parameters).toBeNull();
      expect(t.scanner.y_axis?.tuning_type).toBeNull();
    }

    // Change 2: OPCUA relocated, contents unaffected
    expect(cfg.opcua).toBeDefined();
    expect(cfg.opcua?.client.machine_profile).not.toBeNull();

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL
    const cb0 = cfg.optical_trains[0].optional_components.clearbox as ClearBox;
    const pc0 = cb0.power_characterization!;
    expect(pc0.algorithm_type).toBe('LINEAR');
    expect(pc0.algorithm_equation).toBe('W = a*V + b');
    expect(pc0.input_type).toBe('0-10 V');
    expect(pc0.units_derived_quantity).toBe('Watts');
    expect(pc0.derivation_equation_constants.map((c) => c.name)).toEqual(['b', 'a']);
    expect(pc0.characterization_points.length).toBe(3);

    const cb1 = cfg.optical_trains[1].optional_components.clearbox as ClearBox;
    const pc1 = cb1.power_characterization!;
    expect(pc1.algorithm_type).toBe('POLYNOMIAL');
    expect(pc1.algorithm_equation).toBe('W = c0 + c1*V + c2*V^2');
    expect(pc1.derivation_equation_constants.map((c) => c.name)).toEqual(['c0', 'c1', 'c2']);

    // Change 4: LightSource Power_Characterization — inverse data availability
    const lspc0 = cfg.optical_trains[0].light_source.power_characterization!;
    expect(lspc0.algorithm_type).toBe('LINEAR');
    expect(lspc0.input_type).toBe('Volts');
    expect(lspc0.characterization_points.length).toBe(5);
    expect(lspc0.derivation_equation_constants.length).toBeGreaterThan(0);
  });

  it('toJson() includes firmware_version and power_characterization (explicit serialisation check)', async () => {
    const json = await new Hdf5AdapterV1_1(REFERENCE_V1_1).toJson();
    expect(json).toContain('"firmware_version"');
    expect(json).toContain('"power_characterization"');
  });
});

// ---------------------------------------------------------------------------
// roundtrip
// ---------------------------------------------------------------------------

describe('Hdf5AdapterV1_1 / Hdf5WriterV1_1 — roundtrip', () => {
  it('write -> read -> write -> read, no drift; ClearBox/LightSource cross-wiring guard', async () => {
    const cfg = mockV1_1Config(1);
    const train = cfg.optical_trains[0];
    const cb = train.optional_components.clearbox as ClearBox;
    const newCb: ClearBox = {
      ...cb,
      power_characterization: {
        ...cb.power_characterization!,
        algorithm_type: 'LINEAR',
        derivation_equation_constants: [
          { name: 'b', value: 1.0 },
          { name: 'a', value: 2.0 },
        ] as EquationConstant[],
        characterization_points: [],
      },
    };
    const newLs: LightSource = {
      ...train.light_source,
      power_characterization: {
        ...train.light_source.power_characterization!,
        algorithm_type: 'POLYNOMIAL',
        derivation_equation_constants: [],
        characterization_points: [{ input_value: 9.0, output_value: 99.0 }] as CalibrationPoint[],
      },
    };
    const optional_components: OptionalComponents = { clearbox: newCb };
    cfg.optical_trains = [{ ...train, light_source: newLs, optional_components }];

    const p1 = tempH5Path('v1_1_a');
    await new Hdf5WriterV1_1(cfg).write(p1);
    const mid = await new Hdf5AdapterV1_1(p1).parse();
    const p2 = tempH5Path('v1_1_b');
    await new Hdf5WriterV1_1(mid).write(p2);
    const result = await new Hdf5AdapterV1_1(p2).parse();

    const cbR = result.optical_trains[0].optional_components.clearbox as ClearBox;
    const lsR = result.optical_trains[0].light_source;
    expect(cbR.power_characterization?.algorithm_type).toBe('LINEAR');
    expect(cbR.power_characterization?.derivation_equation_constants.map((c) => c.name)).toEqual(['b', 'a']);
    expect(cbR.power_characterization?.characterization_points).toEqual([]);
    expect(lsR.power_characterization?.algorithm_type).toBe('POLYNOMIAL');
    expect(lsR.power_characterization?.derivation_equation_constants).toEqual([]);
    expect(lsR.power_characterization?.characterization_points[0].input_value).toBe(9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    expect(cbR.power_characterization?.algorithm_type).not.toBe(lsR.power_characterization?.algorithm_type);
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

    migrated.optical_trains.forEach((t, i) => {
      const srcCb = source.optical_trains[i].optional_components.clearbox as ClearBox;
      const cb = t.optional_components.clearbox as ClearBox;
      // Consolidate: same per-train value carried straight through (real
      // fixture already agrees).
      expect(cb.output_path).toBe(srcCb.output_path);
      expect(cb.software_trigger_delay).toBe(srcCb.software_trigger_delay);
      // Addition: no v1.0 source. h5wasm reads an empty/absent string
      // attribute as null (`attrStr`), but the parser normalizes that to
      // undefined for this field specifically — cross_check.py's read-parity
      // phase caught this as a real cross-language bug (2026-09-01): an
      // earlier version of this test asserted `.toBeNull()`, which papered
      // over `firmware_version: attrStr(...)` keeping the key in the JSON
      // output as `null` where Rust/Python/Go/C++ all omit it entirely
      // (skip_serializing_if / omitempty / has_value() / `is not None`).
      // Fixed at the source (`?? undefined`, matching the sibling
      // `power_characterization` field on the very next line) rather than
      // by re-adjusting this assertion a second time.
      expect(cb.firmware_version).toBeUndefined();
      // Removal.
      expect(cb.selected_camera).toBeNull();
      expect(cb.custom_video_format).toBeNull();
      // Change 3: derived ClearBox data has real constants, zero points.
      expect(cb.power_characterization?.characterization_points).toEqual([]);
      expect(cb.power_characterization?.derivation_equation_constants.length).toBeGreaterThan(0);
      expect(cb.power_characterization?.algorithm_type).toBe(srcCb.power_characterization?.algorithm_type);

      const ls = t.light_source;
      // Change 4: derived Light_Source data is the inverse — zero
      // constants, real points.
      expect(ls.power_characterization?.derivation_equation_constants).toEqual([]);
      expect(ls.power_characterization?.characterization_points.length).toBeGreaterThan(0);
    });
  });

  it('Output_Path disagreement raises at write time', async () => {
    const cfg = mockV1Config(2);
    const t1 = cfg.optical_trains[1];
    const cb1: ClearBox = { ...(t1.optional_components.clearbox as ClearBox), output_path: '/other/' };
    cfg.optical_trains = [
      cfg.optical_trains[0],
      { ...t1, optional_components: { clearbox: cb1 } },
    ];
    await expect(writeAs(cfg, tempH5Path('output_path_conflict'), '1.1')).rejects.toThrow(
      /Consolidate conflict on 'Output_Path'/,
    );
  });

  it('Software_Trigger_Delay disagreement raises at write time', async () => {
    const cfg = mockV1Config(2);
    const t1 = cfg.optical_trains[1];
    const cb1: ClearBox = {
      ...(t1.optional_components.clearbox as ClearBox),
      software_trigger_delay: 9999,
    };
    cfg.optical_trains = [
      cfg.optical_trains[0],
      { ...t1, optional_components: { clearbox: cb1 } },
    ];
    await expect(writeAs(cfg, tempH5Path('trigger_delay_conflict'), '1.1')).rejects.toThrow(
      /Consolidate conflict on 'Software_Trigger_Delay'/,
    );
  });

  it('unrecognized Algorithm_Type on ClearBox is best-effort, never raises (Phase 2)', async () => {
    const cfg = mockV1Config(1);
    const t0 = cfg.optical_trains[0];
    const cb0: ClearBox = {
      ...(t0.optional_components.clearbox as ClearBox),
      power_characterization: forwardPowerCharacterizationCoefficients('EXPONENTIAL', '1.5,2.5,3.5'),
    };
    cfg.optical_trains = [{ ...t0, optional_components: { clearbox: cb0 } }];
    const out = tempH5Path('unrecognized_clearbox');
    await writeAs(cfg, out, '1.1');
    const result = await new Hdf5AdapterV1_1(out).parse();
    const pc = (result.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization!;
    expect(pc.algorithm_type).toBe('EXPONENTIAL');
    expect(pc.algorithm_equation).toBeNull();
    expect(pc.derivation_equation_constants.map((c) => c.name)).toEqual(['0', '1', '2']);
    expect(pc.derivation_equation_constants.map((c) => c.value)).toEqual([1.5, 2.5, 3.5]);

    // Round trip: writing back to v1.0 reproduces the original CSV exactly
    // (checked via the structured shape, since the flat field no longer
    // exists to compare against directly).
    const v1_0Out = tempH5Path('unrecognized_clearbox_roundtrip');
    await writeAs(result, v1_0Out, '1.0');
    const back = await new Hdf5AdapterV1_0(v1_0Out).parse();
    const backPc = (back.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization!;
    expect(backPc.algorithm_type).toBe('EXPONENTIAL');
    expect(backPc.derivation_equation_constants.map((c) => c.value)).toEqual([1.5, 2.5, 3.5]);
  });

  it('unrecognized Algorithm_Type on LightSource is best-effort, never raises (Phase 2)', async () => {
    const cfg = mockV1Config(1);
    const t0 = cfg.optical_trains[0];
    const ls0: LightSource = {
      ...t0.light_source,
      power_characterization: forwardPowerCharacterizationPoints('QUADRATIC', '1,2,3,4'),
    };
    cfg.optical_trains = [{ ...t0, light_source: ls0 }];
    const out = tempH5Path('unrecognized_lightsource');
    await writeAs(cfg, out, '1.1');
    const result = await new Hdf5AdapterV1_1(out).parse();
    const pc = result.optical_trains[0].light_source.power_characterization!;
    expect(pc.algorithm_type).toBe('QUADRATIC');
    expect(pc.algorithm_equation).toBeNull();
    expect(pc.characterization_points.length).toBe(2);
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

    const srcCb0 = source.optical_trains[0].optional_components.clearbox as ClearBox;
    const cb0 = back.optical_trains[0].optional_components.clearbox as ClearBox;
    const srcPc0 = srcCb0.power_characterization!;
    const pc0 = cb0.power_characterization!;
    expect(pc0.algorithm_type).toBe(srcPc0.algorithm_type);
    const expected = srcPc0.derivation_equation_constants.map((c) => c.value).sort((a: number, b: number) => a - b);
    const actual = pc0.derivation_equation_constants.map((c) => c.value).sort((a: number, b: number) => a - b);
    expect(actual).toEqual(expected);
    expect(cb0.output_path).toBe(srcCb0.output_path);
    expect(cb0.software_trigger_delay).toBe(srcCb0.software_trigger_delay);

    const srcLs0 = source.optical_trains[0].light_source;
    const ls0 = back.optical_trains[0].light_source;
    const srcLsPc0 = srcLs0.power_characterization!;
    const lsPc0 = ls0.power_characterization!;
    expect(lsPc0.algorithm_type).toBe(srcLsPc0.algorithm_type);
    const expectedPoints = srcLsPc0.characterization_points.flatMap((p) => [p.input_value, p.output_value]);
    const actualPoints = lsPc0.characterization_points.flatMap((p) => [p.input_value, p.output_value]);
    expect(actualPoints).toEqual(expectedPoints);

    // Change 1 Removal fields: lost forever, not restored.
    expect(cb0.selected_camera).toBeNull();
  });

  it('reorders Derivation_Equation_Constants by name before joining as CSV', async () => {
    const cfg = mockV1_1Config(1);
    const t0 = cfg.optical_trains[0];
    const cb0 = t0.optional_components.clearbox as ClearBox;
    const scrambled: EquationConstant[] = [
      { name: 'a', value: 2 },
      { name: 'b', value: 1 },
    ];
    const newCb: ClearBox = {
      ...cb0,
      power_characterization: {
        ...cb0.power_characterization!,
        algorithm_type: 'LINEAR',
        derivation_equation_constants: scrambled,
      },
    };
    cfg.optical_trains = [{ ...t0, optional_components: { clearbox: newCb } }];
    const out = tempH5Path('backward_reorders');
    await writeAs(cfg, out, '1.0');
    const back = await new Hdf5AdapterV1_0(out).parse();
    // Written CSV is "1,2" (b before a); the v1.0 reader forward-derives
    // that back into named constants positionally (b, then a) for LINEAR.
    const backConstants = (back.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization!
      .derivation_equation_constants;
    expect(backConstants).toEqual([
      { name: 'b', value: 1 },
      { name: 'a', value: 2 },
    ]);
  });

  it('writes a blank Volts_To_Watts_Params when Derivation_Equation_Constants is empty', async () => {
    const cfg = mockV1_1Config(1);
    const t0 = cfg.optical_trains[0];
    const cb0 = t0.optional_components.clearbox as ClearBox;
    const newCb: ClearBox = {
      ...cb0,
      power_characterization: { ...cb0.power_characterization!, derivation_equation_constants: [] },
    };
    cfg.optical_trains = [{ ...t0, optional_components: { clearbox: newCb } }];
    const out = tempH5Path('backward_blank_constants');
    await writeAs(cfg, out, '1.0');
    const back = await new Hdf5AdapterV1_0(out).parse();
    // Backward derivation produces "" for Volts_To_Watts_Params (blank, not
    // an error) — a real disk round-trip normalizes an empty string
    // attribute back to null on read, this codebase's standard convention
    // for absent optional strings (confirmed against the other 4
    // languages' identical finding), so forward re-derivation on read
    // produces zero constants.
    expect(
      (back.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization
        ?.derivation_equation_constants,
    ).toEqual([]);
  });

  it('writes a blank Watts_To_Volts_Params when Characterization_Points is empty', async () => {
    const cfg = mockV1_1Config(1);
    const t0 = cfg.optical_trains[0];
    const newLs: LightSource = {
      ...t0.light_source,
      power_characterization: { ...t0.light_source.power_characterization!, characterization_points: [] },
    };
    cfg.optical_trains = [{ ...t0, light_source: newLs }];
    const out = tempH5Path('backward_blank_points');
    await writeAs(cfg, out, '1.0');
    const back = await new Hdf5AdapterV1_0(out).parse();
    expect(back.optical_trains[0].light_source.power_characterization?.characterization_points).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// v1 unaffected
// ---------------------------------------------------------------------------

describe('v1.0 path unaffected by v1.1 existing', () => {
  it('the v1.0 read path forward-derives power_characterization from the flat fields', async () => {
    const config = await new MachineConfigReader(REFERENCE_V1_0).parse();
    expect(config.meta.file_version).toBe('1.0');
    expect(config.optical_trains.length).toBeGreaterThan(0);
    const cb0 = config.optical_trains[0].optional_components.clearbox as ClearBox;
    expect(cb0.power_characterization).toBeTruthy();
    expect(cb0.power_characterization?.algorithm_type).toBe('LINEAR');
  });
});

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

describe('round-trip acceptance criterion', () => {
  it('v1.0 -> v1.1 -> v1.0 reproduces the original exactly', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_0).parse();
    const v1_1Path = tempH5Path('roundtrip_up');
    await writeAs(source, v1_1Path, '1.1');
    const mid = await new MachineConfigReader(v1_1Path).parse();
    const v1_0Path = tempH5Path('roundtrip_down');
    await writeAs(mid, v1_0Path, '1.0');
    const back = await new MachineConfigReader(v1_0Path).parse();

    expect(back.meta.file_version).toBe('1.0');
    back.optical_trains.forEach((t, i) => {
      const srcCb = source.optical_trains[i].optional_components.clearbox as ClearBox;
      const cb = t.optional_components.clearbox as ClearBox;
      expect(cb.output_path).toBe(srcCb.output_path);
      expect(cb.software_trigger_delay).toBe(srcCb.software_trigger_delay);
      const srcPc = srcCb.power_characterization!;
      const pc = cb.power_characterization!;
      expect(pc.algorithm_type).toBe(srcPc.algorithm_type);
      expect(pc.derivation_equation_constants.map((c) => c.value)).toEqual(
        srcPc.derivation_equation_constants.map((c) => c.value),
      );

      const srcLs = source.optical_trains[i].light_source;
      const ls = t.light_source;
      const srcLsPc = srcLs.power_characterization!;
      const lsPc = ls.power_characterization!;
      expect(lsPc.algorithm_type).toBe(srcLsPc.algorithm_type);
      expect(lsPc.characterization_points.flatMap((p) => [p.input_value, p.output_value])).toEqual(
        srcLsPc.characterization_points.flatMap((p) => [p.input_value, p.output_value]),
      );
    });
  });

  it('v1.1 -> v1.0 -> v1.1: fields v1.0 can hold survive, the rest comes back blank', async () => {
    const source = await new MachineConfigReader(REFERENCE_V1_1).parse();
    const v1_0Path = tempH5Path('roundtrip_mirror_down');
    await writeAs(source, v1_0Path, '1.0');
    const mid = await new MachineConfigReader(v1_0Path).parse();
    const v1_1Path = tempH5Path('roundtrip_mirror_up');
    await writeAs(mid, v1_1Path, '1.1');
    const back = await new MachineConfigReader(v1_1Path).parse();

    expect(back.meta.file_version).toBe('1.1');
    back.optical_trains.forEach((t, i) => {
      const srcCb = source.optical_trains[i].optional_components.clearbox as ClearBox;
      const cb = t.optional_components.clearbox as ClearBox;
      const srcPc = srcCb.power_characterization!;
      const pc = cb.power_characterization!;
      expect(pc.algorithm_type).toBe(srcPc.algorithm_type);
      const sortedValues = (constants: EquationConstant[]) => constants.map((c) => c.value).sort((a, b) => a - b);
      expect(sortedValues(pc.derivation_equation_constants)).toEqual(
        sortedValues(srcPc.derivation_equation_constants),
      );
      // Expected loss: v1.1-only fields have no v1.0 round-trip path.
      expect(cb.firmware_version).toBeFalsy();
      expect(pc.input_type).toBeNull();
      expect(pc.units_derived_quantity).toBeNull();
      expect(pc.characterization_points).toEqual([]);
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
    const out = tempH5Path('target_version_up');
    await writeAs(source, out, '1.1');
    const result = await new MachineConfigReader(out).parse();
    expect(result.meta.file_version).toBe('1.1');
    expect(source.meta.file_version).toBe('1.0');

    const v1_1Source = await new MachineConfigReader(REFERENCE_V1_1).parse();
    expect(v1_1Source.meta.file_version).toBe('1.1');
    const out2 = tempH5Path('target_version_down');
    await writeAs(v1_1Source, out2, '1.0');
    const result2 = await new MachineConfigReader(out2).parse();
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
    const out = tempH5Path('dispatcher_v1_1');
    await writeAs(source, out, '1.1');
    const result = await new MachineConfigReader(out).parse();
    expect(result.meta.file_version).toBe('1.1');
    expect((result.optical_trains[0].optional_components.clearbox as ClearBox).output_path).toBe('/recordings/');
    expect(
      (result.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization,
    ).toBeDefined();
  });

  it('routes the natively-authored v1.1 fixture written as v1.0 through the public API', async () => {
    const v1_1Cfg = await new MachineConfigReader(REFERENCE_V1_1).parse();
    const out = tempH5Path('dispatcher_migrated_v1');
    await writeAs(v1_1Cfg, out, '1.0');
    const result = await new MachineConfigReader(out).parse();
    expect(result.meta.file_version).toBe('1.0');
    expect(
      (result.optical_trains[0].optional_components.clearbox as ClearBox).power_characterization?.algorithm_type,
    ).toBe('LINEAR');
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
