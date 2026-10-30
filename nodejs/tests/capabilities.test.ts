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
import { canonicalDigest } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURE = join(__dirname, '..', '..', 'fixtures', 'reference_config.h5');
const REFERENCE_V1_1 = join(__dirname, '..', '..', 'fixtures', 'reference_config_v1_1.h5');
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

  it('save/reopen keeps the OPCUA extension tree of a v1.0 file', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-opcua-'));
    const out = join(dir, 'opcua.h5');
    try {
      const original = await new MachineConfigReader(FIXTURE_OPCUA).parse();
      const opened = await openMachineConfig(FIXTURE_OPCUA);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      expect(isOk(await opened.value.save(out))).toBe(true);
      opened.value.close();

      const again = await new MachineConfigReader(out).parse();
      expect(canonicalDigest(again.extensions!.TM_OPCUA)).toBe(
        canonicalDigest(original.extensions!.TM_OPCUA),
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('the ClearBox extension survives a facade session save', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-cb-'));
    const out = join(dir, 'clearbox.h5');
    try {
      const original = await new MachineConfigReader(FIXTURE).parse();
      const opened = await openMachineConfig(FIXTURE);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      expect(isOk(await opened.value.save(out))).toBe(true);
      opened.value.close();

      const again = await new MachineConfigReader(out).parse();
      expect(canonicalDigest(again.extensions!.ClearBox)).toBe(
        canonicalDigest(original.extensions!.ClearBox),
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('train handles no longer expose optionalComponents or correction accessors', async () => {
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    const train = opened.value.opticalTrain(0);
    expect(train.ok).toBe(true);
    if (train.ok) expect(train.value).not.toHaveProperty('optionalComponents');
    expect(opened.value).not.toHaveProperty('getCorrectionData');
    opened.value.close();
  });

  it('a create()-based instance carries the ClearBox extension without touching disk', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-cd-'));
    const out = join(dir, 'created_clearbox.h5');
    try {
      const created = createMachineConfig('1.0');
      expect(created.ok).toBe(true);
      if (!created.ok) return;
      expect(isOk(await created.value.save(out))).toBe(true);
      created.value.close();
      const again = await new MachineConfigReader(out).parse();
      expect(again.extensions).toHaveProperty('ClearBox');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
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

// ---------------------------------------------------------------------------
// isValid() -- CAPABILITIES_HASH_INTEGRATION_PLAN.md
// ---------------------------------------------------------------------------

describe('isValid()', () => {
  it('is false for the externally-authored reference fixture', async () => {
    // Permanent regression guard, mirroring CONFIGURATION_HASH_PLAN.md's own
    // equivalent at the MachineConfigReader layer: reference_config.h5 was
    // authored externally, so MCF's recomputed hash will never match its
    // stored value. Expected and correct, not a defect.
    const opened = await openMachineConfig(FIXTURE);
    expect(opened.ok).toBe(true);
    if (!opened.ok) return;
    expect(opened.value.isValid()).toBe(false);
    opened.value.close();
  });

  it('is true for a self-written file', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'created.h5');
    try {
      const created = createMachineConfig('1.0');
      expect(created.ok).toBe(true);
      if (!created.ok) return;
      const file = created.value;
      expect(isOk(await file.save(out))).toBe(true);
      expect(file.isValid()).toBe(true);
      file.close();

      const reopened = await openMachineConfig(out);
      expect(reopened.ok).toBe(true);
      if (!reopened.ok) return;
      expect(reopened.value.isValid()).toBe(true);
      reopened.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('flips live across an unsaved edit, then back after save, without reopening', async () => {
    // The test this whole live-method design exists to satisfy: a cached,
    // computed-once-at-open() value could never distinguish these states.
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'created.h5');
    try {
      const created = createMachineConfig('1.0');
      expect(created.ok).toBe(true);
      if (!created.ok) return;
      const file = created.value;
      expect(isOk(await file.save(out))).toBe(true);
      expect(file.isValid()).toBe(true);

      const storedBefore = file.meta().getModel().configuration_hash;
      const train = file.opticalTrain(0);
      expect(train.ok).toBe(true);
      if (!train.ok) return;
      const scanner = train.value.getScanner();
      await train.value.setScanner({ ...scanner, working_distance: 999.5 }, SetMode.Merge);

      // Live-ness: an unsaved edit must be reflected immediately, with no
      // save()/reopen in between. Expected, not a corruption signal (see
      // CAPABILITIES_HASH_INTEGRATION_PLAN.md's documented caveat).
      expect(file.isValid()).toBe(false);

      // The save->live round trip: isValid() must flip back to true from
      // the *same session*, without reopening.
      expect(isOk(await file.save(out))).toBe(true);
      expect(file.isValid()).toBe(true);
      const storedAfter = file.meta().getModel().configuration_hash;
      expect(storedAfter).not.toBe(storedBefore);
      file.close();

      // And a completely fresh session on the same path agrees.
      const reopened = await openMachineConfig(out);
      expect(reopened.ok).toBe(true);
      if (!reopened.ok) return;
      expect(reopened.value.isValid()).toBe(true);
      expect(reopened.value.meta().getModel().configuration_hash).toBe(storedAfter);
      reopened.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('v1.0 save() still routes through real hash computation', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'roundtrip.h5');
    try {
      const opened = await openMachineConfig(FIXTURE);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      const originalStored = opened.value.meta().getModel().configuration_hash;
      expect(isOk(await opened.value.save(out))).toBe(true);
      opened.value.close();

      const rt = await openMachineConfig(out);
      expect(rt.ok).toBe(true);
      if (!rt.ok) return;
      const rtHash = rt.value.meta().getModel().configuration_hash;
      expect(rtHash).toHaveLength(64);
      expect(rtHash).not.toBe(originalStored);
      expect(rt.value.isValid()).toBe(true);
      rt.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('v1.1 save() computes a real hash and updates session state', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mcl-cap-'));
    const out = join(dir, 'v1_1_roundtrip.h5');
    try {
      const opened = await openMachineConfig(REFERENCE_V1_1);
      expect(opened.ok).toBe(true);
      if (!opened.ok) return;
      const file = opened.value;
      expect(file.fileVersion()).toBe('1.1');
      const originalStored = file.meta().getModel().configuration_hash;

      const train = file.opticalTrain(0);
      expect(train.ok).toBe(true);
      if (!train.ok) return;
      const scanner = train.value.getScanner();
      await train.value.setScanner({ ...scanner, working_distance: 555.0 }, SetMode.Merge);
      expect(file.isValid()).toBe(false);

      expect(isOk(await file.save(out))).toBe(true);

      // Live, no reopen:
      expect(file.isValid()).toBe(true);
      const savedHash = file.meta().getModel().configuration_hash;
      expect(savedHash).toHaveLength(64);
      expect(savedHash).not.toBe(originalStored);
      file.close();

      // Fresh session on the written file agrees:
      const reopened = await openMachineConfig(out);
      expect(reopened.ok).toBe(true);
      if (!reopened.ok) return;
      expect(reopened.value.fileVersion()).toBe('1.1');
      expect(reopened.value.isValid()).toBe(true);
      expect(reopened.value.meta().getModel().configuration_hash).toBe(savedHash);
      reopened.value.close();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
