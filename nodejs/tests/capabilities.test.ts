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
  SetMode,
} from '../src/capabilities/index.js';
import { MachineConfigReader } from '../src/reader.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURE = join(__dirname, '..', '..', 'fixtures', 'reference_config.h5');
const FIXTURE_OPCUA = join(
  __dirname,
  '..',
  '..',
  'fixtures',
  'reference_config_opcua.h5',
);

describe('capability facade (File_Version 1.0)', () => {
  it('lists supported versions', () => {
    expect(supportedFileVersions()).toContain('1.0');
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
