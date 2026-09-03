import { describe, it, expect } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import {
  openMachineConfig,
  createMachineConfig,
  supportedFileVersions,
  isOk,
  ok,
  SetMode,
  CREATE,
  type MachineConfigFile,
} from '../src/capabilities/index.js';
import { MachineConfigReader } from '../src/reader.js';
import { MachineConfigWriter } from '../src/writer.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURE = join(__dirname, '..', '..', 'fixtures', 'reference_config.h5');
const FIXTURE_OPCUA = join(
  __dirname,
  '..',
  '..',
  'fixtures',
  'reference_config_opcua.h5',
);
const FIXTURE_OPCUA_MISSING_REQUIRED = join(
  __dirname,
  '..',
  '..',
  'docs',
  'validation',
  'fixtures',
  'opcua_missing_required.h5',
);

describe('capability facade (File_Version 1.0)', () => {
  it('lists supported versions', () => {
    expect(supportedFileVersions()).toContain('1.0');
  });

  it('createMachineConfig rejects an unregistered version', () => {
    const result = createMachineConfig('2.0');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe('UnsupportedVersion');
  });

  it('createMachineConfig dispatches via a registered version', () => {
    // Proves CREATE is genuinely consulted (DISPATCH_REGISTRY_PLAN.md), not
    // just a hardcoded "1.0" check that happens to still work.
    const sentinel = { marker: 'sentinel' } as unknown as MachineConfigFile;
    CREATE['9.9-test'] = () => ok(sentinel);
    try {
      const result = createMachineConfig('9.9-test');
      expect(result.ok).toBe(true);
      if (result.ok) expect(result.value).toBe(sentinel);
    } finally {
      delete CREATE['9.9-test'];
    }
  });

  it('open → getScanner fields match reader JSON', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const file = opened.value;
    expect(file.fileVersion()).toBe('1.0');

    const reader = new MachineConfigReader(FIXTURE);
    const json = await reader.parse();
    const train = file.opticalTrain(0);
    expect(train.ok).toBe(true);
    if (!train.ok) return;
    const scanner = train.value.getScanner();
    expect(scanner.working_distance).toBe(json.optical_trains[0].scanner.working_distance);
    expect(scanner.manufacturer).toBe(json.optical_trains[0].scanner.manufacturer);
    file.close();
  });

  it('Merge setScanner preserves untouched fields after save/reopen', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'merge.h5');
    try {
      const opened = await openMachineConfig(FIXTURE);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      const file = opened.value;
      const train = file.opticalTrain(0);
      expect(train.ok).toBe(true);
      if (!train.ok) return;
      const before = train.value.getScanner();
      const manufacturer = before.manufacturer;
      await train.value.setScanner(
        { ...before, working_distance: 123.5 },
        SetMode.Merge,
      );
      expect(isOk(await file.save(out))).toBe(true);
      file.close();

      const again = await openMachineConfig(out);
      expect(again.ok).toBe(true);
      if (!again.ok) return;
      const sc = again.value.opticalTrain(0);
      expect(sc.ok).toBe(true);
      if (!sc.ok) return;
      const after = sc.value.getScanner();
      expect(after.working_distance).toBe(123.5);
      expect(after.manufacturer).toBe(manufacturer);
      again.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('Replace setScanner replaces whole node including extra', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const train = opened.value.opticalTrain(0);
    expect(train.ok).toBe(true);
    if (!train.ok) return;
    const before = train.value.getScanner();
    const replacement = {
      manufacturer: 'ReplaceCo',
      model: 'ReplaceModel',
      serial_number: 'R-1',
      working_distance: 1,
      working_distance_unit: 'mm',
      scan_field_x: 1,
      scan_field_x_unit: 'mm',
      scan_field_y: 1,
      scan_field_y_unit: 'mm',
      scan_field_z: 1,
      scan_field_z_unit: 'mm',
      scan_head_offset_x: 0,
      scan_head_offset_x_unit: 'mm',
      scan_head_offset_y: 0,
      scan_head_offset_y_unit: 'mm',
      scan_head_offset_z: 0,
      scan_head_offset_z_unit: 'mm',
      extra: {},
    };
    const set = train.value.setScanner(replacement as typeof before, SetMode.Replace);
    expect(set.ok).toBe(true);
    const after = train.value.getScanner();
    expect(after.manufacturer).toBe('ReplaceCo');
    expect(after.working_distance).toBe(1);
    expect(after.extra ?? {}).toEqual({});
    expect(after.model).not.toBe(before.model);
    opened.value.close();
  });

  it('opticalTrains length and InvalidIndex', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    expect(opened.value.opticalTrains().length).toBeGreaterThanOrEqual(1);
    const bad = opened.value.opticalTrain(999);
    expect(bad.ok).toBe(false);
    if (!bad.ok) expect(bad.error.code).toBe('InvalidIndex');
    opened.value.close();
  });

  it('opcua NotPresent vs present', async () => {
    const noOpc = await openMachineConfig(FIXTURE);
    expect(noOpc.ok).toBe(true);
    if (!noOpc.ok) return;
    const missing = noOpc.value.opcua();
    expect(missing.ok).toBe(false);
    if (!missing.ok) expect(missing.error.code).toBe('NotPresent');
    noOpc.value.close();

    const withOpc = await openMachineConfig(FIXTURE_OPCUA);
    expect(withOpc.ok).toBe(true);
    if (!withOpc.ok) return;
    const present = withOpc.value.opcua();
    expect(present.ok).toBe(true);
    if (present.ok) {
      const model = present.value.getModel();
      expect(model).toBeTruthy();
    }
    withOpc.value.close();
  });

  it('opcua() Ok when all required fields present on reference_opcua fixture', async () => {
    const opened = await openMachineConfig(FIXTURE_OPCUA);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const result = opened.value.opcua();
    expect(result.ok).toBe(true);
    if (result.ok) {
      const model = result.value.getModel();
      expect(model.client.machine_profile).not.toBeNull();
      expect(Object.values(model.triggers).every((t) => t.event != null)).toBe(true);
    }
    opened.value.close();
  });

  it('opcua() reports all seven missing required fields at once', async () => {
    const opened = await openMachineConfig(FIXTURE_OPCUA_MISSING_REQUIRED);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const result = opened.value.opcua();
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.code).toBe('ValidationError');

    const expected = new Set([
      'Machine_Profile',
      'Root_Node',
      'Configure_Client',
      'Pipe_Name',
      'Triggers_Enabled',
      'Trigger_Stop_Ceiling_Layers',
      'Laser Emission Interlock.Event',
    ]);
    expect(new Set(result.error.details)).toEqual(expected);
    expect(result.error.details?.some((d) => d.startsWith('Chamber Oxygen Level'))).toBe(false);
    opened.value.close();
  });

  it('opcua() never reports an optional field, even when genuinely absent', async () => {
    // opcua_missing_required.h5 only clears the 7 required fields — every
    // optional field is still present there, so absence of an optional field
    // from `details` would be trivially true. Also clear an optional field
    // (keep_alive_count) in memory, re-write to a temp file, and confirm
    // `details` still names exactly the same 7 items, not 8.
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-opcua-'));
    const out = join(dir, 'missing_required_plus_optional.h5');
    try {
      const reader = new MachineConfigReader(FIXTURE_OPCUA_MISSING_REQUIRED);
      const config = await reader.parse();
      config.opcua!.client.keep_alive_count = null;
      await new MachineConfigWriter(config).write(out);

      const opened = await openMachineConfig(out);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      const result = opened.value.opcua();
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.details?.some((d) => d.includes('Keep_Alive_Count'))).toBe(false);
      expect(result.error.details).toHaveLength(7);
      opened.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('optionalComponents null vs clearbox present', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const train = opened.value.opticalTrain(0);
    expect(train.ok).toBe(true);
    if (!train.ok) return;
    const oc = train.value.optionalComponents();
    expect(oc).not.toBeNull();
    if (!oc) return;
    const cb = oc.clearbox();
    expect(cb.ok).toBe(true);
    opened.value.close();

    const created = createMachineConfig('1.0');
    expect(created.ok).toBe(true);
    if (!created.ok) return;
    // Mock builder defaults include clearbox; strip it to assert null path.
    const t0 = created.value.opticalTrain(0);
    expect(t0.ok).toBe(true);
    if (!t0.ok) return;
    const model = t0.value.getModel();
    (model as { optional_components: { clearbox: null } }).optional_components = {
      clearbox: null,
    };
    t0.value.setModel(model, SetMode.Replace);
    expect(t0.value.optionalComponents()).toBeNull();
    created.value.close();
  });

  it('getCorrectionData/getInverseCorrectionData match Reader', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const file = opened.value;

    const reader = new MachineConfigReader(FIXTURE);
    const expected = await reader.getCorrectionData(0);
    const result = file.getCorrectionData(0);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.shape).toEqual(expected.shape);
    expect(Array.from(result.value.data)).toEqual(Array.from(expected.data));

    const expectedInv = await reader.getInverseCorrectionData(0);
    const resultInv = file.getInverseCorrectionData(0);
    expect(resultInv.ok).toBe(true);
    if (!resultInv.ok) return;
    expect(resultInv.value.shape).toEqual(expectedInv.shape);
    expect(Array.from(resultInv.value.data)).toEqual(Array.from(expectedInv.data));
    file.close();
  });

  it('getCorrectionData shape and NaN present through facade', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const grid = opened.value.getCorrectionData(0);
    expect(grid.ok).toBe(true);
    if (!grid.ok) return;
    expect(grid.value.shape).toEqual([257, 257, 2]);
    expect(Array.from(grid.value.data).some((v) => Number.isNaN(v))).toBe(true);
    opened.value.close();
  });

  it('getCorrectionData missing clearbox is NotPresent', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-cd-'));
    const out = join(dir, 'no_clearbox.h5');
    try {
      // Every stock fixture's trains have a ClearBox, so build one without:
      // take the reference config, strip train 0's ClearBox, re-write.
      const reader = new MachineConfigReader(FIXTURE);
      const config = await reader.parse();
      config.optical_trains[0].optional_components.clearbox = null;
      await new MachineConfigWriter(config).write(out);

      const opened = await openMachineConfig(out);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      const result = opened.value.getCorrectionData(0);
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.error.code).toBe('NotPresent');
      const invResult = opened.value.getInverseCorrectionData(0);
      expect(invResult.ok).toBe(false);
      if (!invResult.ok) expect(invResult.error.code).toBe('NotPresent');
      opened.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('getCorrectionData works on create()-based instance without touching disk', () => {
    // The specific case that rules out delegate-to-Reader-by-reopening: a
    // create()-d facade has no path at all, so this must convert the
    // already-loaded in-memory model, not re-read from anywhere.
    const created = createMachineConfig('1.0');
    expect(created.ok).toBe(true);
    if (!created.ok) return;
    const file = created.value;
    const grid = file.getCorrectionData(0);
    expect(grid.ok).toBe(true);
    if (grid.ok) expect(grid.value.shape).toEqual([257, 257, 2]);
    const invGrid = file.getInverseCorrectionData(0);
    expect(invGrid.ok).toBe(true);
    if (invGrid.ok) expect(invGrid.value.shape).toEqual([257, 257, 2]);
    file.close();
  });

  it('create(1.0) → set machine name → save → reopen keeps version', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'created.h5');
    try {
      const created = createMachineConfig('1.0');
      expect(created.ok).toBe(true);
      if (!created.ok) return;
      const file = created.value;
      expect(file.fileVersion()).toBe('1.0');
      const meta = file.meta().getModel();
      file.meta().setModel({ ...meta, machine_name: 'CreatedMachine' }, SetMode.Merge);
      expect(isOk(await file.save(out))).toBe(true);
      file.close();

      const again = await openMachineConfig(out);
      expect(again.ok).toBe(true);
      if (!again.ok) return;
      expect(again.value.fileVersion()).toBe('1.0');
      expect(again.value.meta().getModel().machine_name).toBe('CreatedMachine');
      again.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
