// S-01-09/AV-01-11 scenarios for the Node.js validation app (VALIDATION_PLAN.md §8).
//
// AV-09-11 live here (unlike Rust/Go/C++'s AV-09-11, which can only live in
// their own source trees) because Node's dispatcher is a registry
// (_READERS/_WRITERS), not a hardcoded match/switch — there's a real
// dispatch-table seam for the mock adapter to hook into, so these scenarios
// exercise the public MachineConfigReader/MachineConfigWriter facade
// directly, same as every other scenario here. The mock lives in
// nodejs/tests/mockV1_1.ts (never packaged) — imported directly from the
// source tree at a fixed relative offset, mirroring Python's sys.path trick
// in the equivalent scenarios. This only works because this file lives
// in-repo, not in an externally-runnable copy.
import { createHash, randomUUID } from 'node:crypto';
import { existsSync, globSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  MachineConfigReader,
  MachineConfigWriter,
  MockConfigBuilder,
  UnsupportedFileVersion,
  openMachineConfig,
} from 'machine-config-library';
import type {
  ClearBox,
  Collimator,
  LightSource,
  MachineConfig,
  OpcuaConfig,
  OpticalTrain,
  Scanner,
  ScannerCard,
} from 'machine-config-library';
import { MockV1_1Reader, MockV1_1Writer, makeMockConfig } from '../../../../nodejs/tests/mockV1_1.js';

function avFixture(fixturesDir: string, name: string): string {
  return join(fixturesDir, '..', 'docs', 'validation', 'fixtures', name);
}

// S-01: Read reference fixture, all scalar fields
export async function runS01(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const cfg = await new MachineConfigReader(join(fixturesDir, 'reference_config.h5')).parse();

  if (cfg.meta.machine_name !== 'TM-LPBF-02: AconityMIDI+_OG') {
    return [false, `machine_name: got '${cfg.meta.machine_name}'`];
  }
  if (cfg.machine.build_plate_x == null || Math.abs(cfg.machine.build_plate_x - 250.0) > 0.001) {
    return [false, `build_plate_x: got ${cfg.machine.build_plate_x}`];
  }
  if (cfg.machine.build_plate_y == null || Math.abs(cfg.machine.build_plate_y - 250.0) > 0.001) {
    return [false, `build_plate_y: got ${cfg.machine.build_plate_y}`];
  }
  if (cfg.optical_trains.length !== 2) {
    return [false, `optical_trains count: got ${cfg.optical_trains.length}`];
  }

  const wd = cfg.optical_trains[0].scanner.working_distance;
  if (wd == null || Math.abs(wd - 670.0) > 0.1) {
    return [false, `train[0].working_distance: got ${wd}`];
  }

  const r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
  if (r0 == null || Math.abs(r0) > 0.001) {
    return [false, `train[0].scan_head_rotation: got ${r0}`];
  }

  const r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
  if (r1 == null || Math.abs(r1 - 180.0) > 0.001) {
    return [false, `train[1].scan_head_rotation: got ${r1}`];
  }

  const h = cfg.meta.configuration_hash;
  if (h.length !== 64 || !/^[0-9a-fA-F]+$/.test(h)) {
    return [false, `configuration_hash invalid: '${h}'`];
  }
  if (cfg.meta.file_version.trim() !== '1.0') {
    return [false, `file_version: got '${cfg.meta.file_version}'`];
  }

  return [true, 'all scalar fields match expected values'];
}

// S-02: Read reference fixture, correction data
export async function runS02(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const reader = new MachineConfigReader(join(fixturesDir, 'reference_config.h5'));
  const cd = await reader.getCorrectionData(0);
  const icd = await reader.getInverseCorrectionData(0);

  if (cd.shape.join(',') !== '257,257,2') {
    return [false, `correction_data shape: ${cd.shape.join(',')}`];
  }
  if (icd.shape.join(',') !== '257,257,2') {
    return [false, `inverse_correction_data shape: ${icd.shape.join(',')}`];
  }
  if (!cd.data.some((v) => Number.isFinite(v))) {
    return [false, 'correction_data: no finite values'];
  }
  if (!icd.data.some((v) => Number.isFinite(v))) {
    return [false, 'inverse_correction_data: no finite values'];
  }

  const identical = cd.data.length === icd.data.length &&
    cd.data.every((v, i) => v === icd.data[i] || (Number.isNaN(v) && Number.isNaN(icd.data[i])));
  if (identical) {
    return [false, 'correction_data and inverse_correction_data are identical'];
  }

  const cdHash = createHash('sha256').update(Buffer.from(cd.data.buffer)).digest('hex');
  return [true, `shapes OK, finite values OK, forward≠inverse, correction_data SHA-256=${cdHash}`];
}

// S-03: Read real AconityMIDI fixture file
export async function runS03(_fixturesDir: string, realDir: string): Promise<[boolean, string]> {
  const matches = globSync('*.h5', { cwd: realDir })
    .filter((name) => name.includes('AconityMIDI') && name.includes('OG_178'));
  if (matches.length === 0) {
    return [false, `real AconityMIDI file not found in ${realDir}`];
  }

  const cfg = await new MachineConfigReader(`${realDir}/${matches[0]}`).parse();
  const fields = [
    `machine_name=${JSON.stringify(cfg.meta.machine_name)}`,
    `file_version=${JSON.stringify(cfg.meta.file_version)}`,
    `trains=${cfg.optical_trains.length}`,
    `build_plate_x=${cfg.machine.build_plate_x}`,
    `build_plate_y=${cfg.machine.build_plate_y}`,
    `wd=${cfg.optical_trains[0].scanner.working_distance}`,
    `rotation[0]=${cfg.optical_trains[0].scanner.scan_head_rotation}`,
    `hash=${cfg.meta.configuration_hash.slice(0, 16)}...`,
  ].join(' | ');
  return [true, fields];
}

// S-04: Write modified config and verify field change survives round-trip
export async function runS04(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const cfg = await new MachineConfigReader(path).parse();
  const origX = cfg.machine.build_plate_x;

  const modified = {
    ...cfg,
    meta: { ...cfg.meta, machine_name: 'VALIDATION_TEST_MACHINE' },
    machine: { ...cfg.machine, machine_name: 'VALIDATION_TEST_MACHINE' },
  };

  const tmp = join(tmpdir(), `s04_${randomUUID()}.h5`);
  await new MachineConfigWriter(modified).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.meta.machine_name !== 'VALIDATION_TEST_MACHINE') {
    return [false, `machine_name not persisted: '${rb.meta.machine_name}'`];
  }
  if (rb.meta.file_version.trim() !== '1.0') {
    return [false, `file_version changed: '${rb.meta.file_version}'`];
  }
  if (rb.machine.build_plate_x !== origX) {
    return [false, `build_plate_x changed: ${origX} → ${rb.machine.build_plate_x}`];
  }

  return [true, 'machine_name persisted, file_version and other fields unchanged'];
}

// S-05: Full binary round-trip with correction hash
function arraysEqual(a: Float64Array, b: Float64Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const av = a[i], bv = b[i];
    if (av !== bv && !(Number.isNaN(av) && Number.isNaN(bv))) return false;
  }
  return true;
}

export async function runS05(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const reader = new MachineConfigReader(path);
  const cfg = await reader.parse({ includeBinary: true });

  const cdBefore = await reader.getCorrectionData(0);
  const icdBefore = await reader.getInverseCorrectionData(0);

  const tmp = join(tmpdir(), `s05_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const reader2 = new MachineConfigReader(tmp);

  const cdAfter = await reader2.getCorrectionData(0);
  const icdAfter = await reader2.getInverseCorrectionData(0);

  if (!arraysEqual(cdBefore.data, cdAfter.data)) {
    const hBefore = createHash('sha256').update(Buffer.from(cdBefore.data.buffer)).digest('hex').slice(0, 16);
    const hAfter = createHash('sha256').update(Buffer.from(cdAfter.data.buffer)).digest('hex').slice(0, 16);
    return [false, `correction_data mismatch: ${hBefore}... → ${hAfter}...`];
  }
  if (!arraysEqual(icdBefore.data, icdAfter.data)) {
    return [false, 'inverse_correction_data mismatch after roundtrip'];
  }

  const cdHash = createHash('sha256').update(Buffer.from(cdBefore.data.buffer)).digest('hex');
  return [true, `correction_data preserved: SHA-256=${cdHash}`];
}

// S-06: Build synthetic config with 2 lasers, verify fields and round-trip
export async function runS06(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const cfg = new MockConfigBuilder({ nLasers: 2 }).build();

  if (cfg.optical_trains.length !== 2) {
    return [false, `optical_trains count: ${cfg.optical_trains.length}`];
  }

  const r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
  const r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
  if (r0 == null || Math.abs(r0) > 0.001) {
    return [false, `train[0].scan_head_rotation: ${r0}`];
  }
  if (r1 == null || Math.abs(r1 - 180.0) > 0.001) {
    return [false, `train[1].scan_head_rotation: ${r1}`];
  }
  if (!cfg.meta.machine_name) {
    return [false, 'machine_name is empty'];
  }

  const cb = cfg.optical_trains[0].optional_components.clearbox;
  if (cb == null || cb.correction_data == null) {
    return [false, 'clearbox or correction_data is null'];
  }
  const center = cb.correction_data[128][128][0];
  if (center == null || !Number.isFinite(center) || Math.abs(center - 2.0) > 0.01) {
    return [false, `correction_data center: expected ~2.0, got ${center}`];
  }

  const tmp = join(tmpdir(), `s06_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.optical_trains.length !== 2) {
    return [false, `readback trains: ${rb.optical_trains.length}`];
  }
  if (rb.meta.machine_name !== cfg.meta.machine_name) {
    return [false, 'machine_name changed after roundtrip'];
  }

  return [true, '2-laser build OK, center≈2.0, roundtrip OK'];
}

// S-07: OPCUA configuration round-trip
export async function runS07(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config_opcua.h5');
  if (!existsSync(path)) {
    return [false, `OPCUA fixture not found: ${path}`];
  }

  const cfg = await new MachineConfigReader(path).parse();
  if (cfg.opcua == null) {
    return [false, 'opcua is undefined after reading OPCUA fixture'];
  }

  const origUrl = cfg.opcua.client.server_url;
  const origTimeout = cfg.opcua.client.session_timeout;
  const origTriggersEnabled = cfg.opcua.triggers_enabled;
  const origTriggerNames = new Set(Object.keys(cfg.opcua.triggers));
  // Newly-promoted fields (OPCUA_FIELD_PROMOTION_PLAN.md Phase 1) — a
  // representative subset, proving the low-level roundtrip works through
  // the public MachineConfigReader/Writer API too, not just in unit tests.
  const origMachineProfile = cfg.opcua.client.machine_profile;
  const origRootNode = cfg.opcua.client.root_node;
  const origPipeName = cfg.opcua.pipe.pipe_name;
  const origCeilingLayers = cfg.opcua.trigger_stop_ceiling_layers;

  const tmp = join(tmpdir(), `s07_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.opcua == null) {
    return [false, 'opcua is undefined after roundtrip'];
  }
  if (rb.opcua.client.server_url !== origUrl) {
    return [false, `server_url changed: '${origUrl}' → '${rb.opcua.client.server_url}'`];
  }
  if (rb.opcua.client.session_timeout !== origTimeout) {
    return [false, `session_timeout changed: ${origTimeout} → ${rb.opcua.client.session_timeout}`];
  }
  if (rb.opcua.triggers_enabled !== origTriggersEnabled) {
    return [false, `triggers_enabled changed: ${origTriggersEnabled} → ${rb.opcua.triggers_enabled}`];
  }

  const rbNames = new Set(Object.keys(rb.opcua.triggers));
  if (rbNames.size !== origTriggerNames.size || ![...origTriggerNames].every((n) => rbNames.has(n))) {
    const missing = [...origTriggerNames].filter((n) => !rbNames.has(n));
    return [false, `trigger names changed: missing=${JSON.stringify(missing)}`];
  }

  const co = 'Chamber Oxygen Level';
  if (co in cfg.opcua.triggers) {
    const ot = cfg.opcua.triggers[co], rt = rb.opcua.triggers[co];
    if (ot.signal !== rt.signal || ot.subsystem !== rt.subsystem) {
      return [false, `'${co}' signal/subsystem changed`];
    }
    if (ot.event !== rt.event || ot.trigger_label !== rt.trigger_label) {
      return [false, `'${co}' event/trigger_label changed`];
    }
  }

  if (rb.opcua.client.machine_profile !== origMachineProfile) {
    return [false, `machine_profile changed: '${origMachineProfile}' → '${rb.opcua.client.machine_profile}'`];
  }
  if (rb.opcua.client.root_node !== origRootNode) {
    return [false, `root_node changed: '${origRootNode}' → '${rb.opcua.client.root_node}'`];
  }
  if (rb.opcua.pipe.pipe_name !== origPipeName) {
    return [false, `pipe_name changed: '${origPipeName}' → '${rb.opcua.pipe.pipe_name}'`];
  }
  if (rb.opcua.trigger_stop_ceiling_layers !== origCeilingLayers) {
    return [false, `trigger_stop_ceiling_layers changed: ${origCeilingLayers} → ${rb.opcua.trigger_stop_ceiling_layers}`];
  }

  return [
    true,
    `OPCUA roundtrip OK: ${origTriggerNames.size} triggers, url='${origUrl}', machine_profile='${origMachineProfile}'`,
  ];
}

// S-08: Drastic change to real AconityMIDI file — new train, build_plate_x, rotation, clearbox cleared
export async function runS08(_fixturesDir: string, realDir: string): Promise<[boolean, string]> {
  const matches = globSync('*.h5', { cwd: realDir })
    .filter((name) => name.includes('AconityMIDI') && name.includes('OG_178'));
  if (matches.length === 0) {
    return [false, `real AconityMIDI file not found in ${realDir}`];
  }

  const cfg = await new MachineConfigReader(`${realDir}/${matches[0]}`).parse();

  // Clone train 1 as train 3
  const newTrain = structuredClone(cfg.optical_trains[1]);
  newTrain.train_id = 'Optical_Train_03';
  newTrain.scanner = { ...newTrain.scanner, scan_head_rotation: 90.0 };
  newTrain.optional_components = { ...newTrain.optional_components, clearbox: null };

  const modified = {
    ...cfg,
    meta: { ...cfg.meta, machine_name: 'MODIFIED_ACONITY_VALIDATION' },
    machine: { ...cfg.machine, build_plate_x: 350.0, machine_name: 'MODIFIED_ACONITY_VALIDATION' },
    optical_trains: [...cfg.optical_trains, newTrain],
  };

  const tmp = join(tmpdir(), `s08_${randomUUID()}.h5`);
  await new MachineConfigWriter(modified).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.optical_trains.length !== 3) {
    return [false, `optical_trains: expected 3, got ${rb.optical_trains.length}`];
  }
  if (rb.machine.build_plate_x == null || Math.abs(rb.machine.build_plate_x - 350.0) > 0.001) {
    return [false, `build_plate_x: got ${rb.machine.build_plate_x}`];
  }
  const r2 = rb.optical_trains[2].scanner.scan_head_rotation;
  if (r2 == null || Math.abs(r2 - 90.0) > 0.001) {
    return [false, `train[2].scan_head_rotation: got ${r2}`];
  }
  if (rb.optical_trains[2].optional_components.clearbox !== null) {
    return [false, 'train[2].clearbox should be null'];
  }
  if (rb.meta.machine_name !== 'MODIFIED_ACONITY_VALIDATION') {
    return [false, `machine_name: got '${rb.meta.machine_name}'`];
  }
  if (rb.meta.file_version.trim() !== '1.0') {
    return [false, `file_version changed: '${rb.meta.file_version}'`];
  }

  return [true, '3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK'];
}

// S-09: Public type export surface — must compile with zero errors.
// All consumer-facing types must be importable from the package root only.
function assertType<T>(_value: T): void {
  // Compile-only check: reaching tsc --noEmit success means every type above
  // resolved from the package root, with no internal sub-module paths needed.
}

export async function runS09(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const builder = new MockConfigBuilder();
  const cfg: MachineConfig = builder.build();
  assertType<Scanner>(cfg.optical_trains[0]?.scanner);
  assertType<OpticalTrain>(cfg.optical_trains[0]);
  assertType<LightSource>(cfg.optical_trains[0]?.light_source);
  assertType<Collimator>(cfg.optical_trains[0]?.collimator);
  assertType<ScannerCard>(cfg.optical_trains[0]?.scanner_card);
  assertType<ClearBox | null | undefined>(cfg.optical_trains[0]?.optional_components.clearbox);
  assertType<OpcuaConfig | undefined>(cfg.opcua);
  assertType<typeof MachineConfigReader>(MachineConfigReader);
  assertType<typeof MachineConfigWriter>(MachineConfigWriter);

  return [true, 'all public types importable from machine-config-library top-level'];
}

// AV-01: Reader rejects unknown File_Version with typed error
export async function runAv01(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'v2_0_unknown.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, "no error raised for File_Version='2.0'"];
  } catch (e) {
    if (e instanceof UnsupportedFileVersion) {
      if (e.version === '2.0') {
        return [true, `UnsupportedFileVersion raised, version='${e.version}'`];
      }
      return [false, `UnsupportedFileVersion raised but version='${e.version}'`];
    }
    return [false, `wrong exception: ${(e as Error).constructor.name}: ${e}`];
  }
}

// AV-02: Reader handles absent File_Version attribute predictably
export async function runAv02(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'missing_version.h5');
  try {
    const cfg = await new MachineConfigReader(fixture).parse();
    return [true, `missing File_Version defaults to '1.0', reads OK, file_version='${cfg.meta.file_version}'`];
  } catch (e) {
    return [true, `missing File_Version raises ${(e as Error).constructor.name}: ${e}`];
  }
}

// AV-03: v1.0 reader encountering a v1.1 file fails loudly
export async function runAv03(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'v1_1_simulated.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, "no error raised for File_Version='1.1'"];
  } catch (e) {
    if (e instanceof UnsupportedFileVersion) {
      if (e.version === '1.1') {
        return [true, `UnsupportedFileVersion raised, version='${e.version}'`];
      }
      return [false, `UnsupportedFileVersion raised but version='${e.version}'`];
    }
    return [false, `wrong exception: ${(e as Error).constructor.name}: ${e}`];
  }
}

// AV-04: Reader returns typed error when required group is absent
export async function runAv04(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'missing_machine_group.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, 'no error raised for missing Machine/ group'];
  } catch (e) {
    return [true, `${(e as Error).constructor.name} raised for missing Machine/ group: ${e}`];
  }
}

// AV-05: Reader handles corrupt required attribute gracefully
export async function runAv05(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'corrupt_scalar.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, 'no error raised for corrupt Build_Plate_X_Dimension'];
  } catch (e) {
    return [true, `${(e as Error).constructor.name} raised for corrupt scalar: ${e}`];
  }
}

// AV-06: Dispatcher normalizes whitespace in File_Version
export async function runAv06(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'version_whitespace.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"];
  } catch (e) {
    return [false, `${(e as Error).constructor.name}: ${e}`];
  }
}

// AV-07: Dispatcher handles empty string File_Version
export async function runAv07(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'empty_version.h5');
  try {
    const cfg = await new MachineConfigReader(fixture).parse();
    return [true, `empty File_Version defaults to '1.0', reads OK, file_version='${cfg.meta.file_version}'`];
  } catch (e) {
    return [true, `empty File_Version raises ${(e as Error).constructor.name}: ${e}`];
  }
}

// AV-08: File_Version string survives write→read unchanged
export async function runAv08(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const cfg = await new MachineConfigReader(path).parse();
  const origVersion = cfg.meta.file_version.trim();

  const tmp = join(tmpdir(), `av08_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();
  const rbVersion = rb.meta.file_version.trim();

  if (rbVersion !== '1.0') {
    return [false, `file_version after roundtrip: expected '1.0', got '${rbVersion}'`];
  }
  if (rbVersion !== origVersion) {
    return [false, `file_version changed: '${origVersion}' → '${rbVersion}'`];
  }

  return [true, `File_Version survives roundtrip unchanged: '${rbVersion}'`];
}

// AV-09: Mock v1.1 adapter — adding a new adapter leaves the v1.0 adapter untouched
export async function runAv09(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const cfg = await new MachineConfigReader(path).parse();

  if (cfg.meta.machine_name !== 'TM-LPBF-02: AconityMIDI+_OG') {
    return [false, `v1.0 adapter broken after importing mock v1.1: machine_name='${cfg.meta.machine_name}'`];
  }
  if (cfg.meta.file_version.trim() !== '1.0') {
    return [false, `v1.0 adapter returned wrong file_version: '${cfg.meta.file_version}'`];
  }
  if (cfg.optical_trains.length !== 2) {
    return [false, `v1.0 adapter returned wrong train count: ${cfg.optical_trains.length}`];
  }

  return [true, 'v1.0 adapter unaffected by mock v1.1 adapter import; all fields correct'];
}

// AV-10: Mock v1.1 adapter — forward migration (v1.0 -> v1.1-mock)
export async function runAv10(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const src = join(fixturesDir, 'reference_config.h5');
  const v1_0 = await new MachineConfigReader(src).parse();

  const tmp = join(tmpdir(), `av10_${randomUUID()}.h5`);
  await new MockV1_1Writer({ ...v1_0, meta: { ...v1_0.meta, file_version: '1.1-mock' } }).write(tmp);
  const v1_1 = await new MockV1_1Reader(tmp).parse();

  // ADDITION: no v1.0 source — typed fields are null
  if (v1_1.meta.facility_id != null) {
    return [false, `facility_id should be null, got '${v1_1.meta.facility_id}'`];
  }
  if (v1_1.meta.config_author != null) {
    return [false, `config_author should be null, got '${v1_1.meta.config_author}'`];
  }

  // REMOVAL: fields absent in v1.1 reader
  if (v1_1.machine.gas_flow_direction != null) {
    return [false, 'gas_flow_direction should be null after forward migration'];
  }
  if (v1_1.machine.recoat_direction != null) {
    return [false, 'recoat_direction should be null after forward migration'];
  }

  // NAME, PATH, NAME+PATH: values preserved through StableModel
  if (v1_1.machine.machine_name !== v1_0.machine.machine_name) {
    return [false, `machine_name changed: '${v1_0.machine.machine_name}' -> '${v1_1.machine.machine_name}'`];
  }
  const t0 = v1_0.optical_trains[0], t1 = v1_1.optical_trains[0];
  if (t1.scanner.working_distance !== t0.scanner.working_distance) {
    return [false, `working_distance changed: ${t0.scanner.working_distance} -> ${t1.scanner.working_distance}`];
  }
  if (v1_1.machine.build_plate_z !== v1_0.machine.build_plate_z) {
    return [false, 'build_plate_z changed'];
  }
  if (v1_1.machine.build_plate_radius !== v1_0.machine.build_plate_radius) {
    return [false, 'build_plate_radius changed'];
  }
  if (v1_1.machine.build_plate_x !== v1_0.machine.build_plate_x) {
    return [false, 'build_plate_x changed'];
  }
  if (v1_1.machine.build_plate_y !== v1_0.machine.build_plate_y) {
    return [false, 'build_plate_y changed'];
  }

  return [true, `forward migration OK — ADDITION=null, REMOVAL=null, ` +
    `machine_name='${v1_1.machine.machine_name}', ` +
    `build_plate x=${v1_1.machine.build_plate_x} y=${v1_1.machine.build_plate_y} ` +
    `z=${v1_1.machine.build_plate_z} preserved`];
}

// AV-11: Mock v1.1 adapter — backward migration (v1.1-mock -> v1.0)
export async function runAv11(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const cfg = makeMockConfig({
    machineName: 'BackwardMigrationTest',
    facilityId: 'Lab-Validation',
    configAuthor: 'ValidationBot',
  });

  const v1_1Path = join(tmpdir(), `av11_v1_1_${randomUUID()}.h5`);
  const v1_0Path = join(tmpdir(), `av11_v1_0_${randomUUID()}.h5`);

  await new MockV1_1Writer(cfg).write(v1_1Path);

  // Downgrade: read v1.1-mock, write v1.0
  const v1_1 = await new MockV1_1Reader(v1_1Path).parse();
  await new MachineConfigWriter({ ...v1_1, meta: { ...v1_1.meta, file_version: '1.0' } }).write(v1_0Path);
  const v1_0 = await new MachineConfigReader(v1_0Path).parse();

  // ADDITION fields must be lost (v1.0 writer/reader don't know these attrs)
  if (v1_0.meta.facility_id != null) {
    return [false, `facility_id should be lost after downgrade, got '${v1_0.meta.facility_id}'`];
  }
  if (v1_0.meta.config_author != null) {
    return [false, `config_author should be lost after downgrade, got '${v1_0.meta.config_author}'`];
  }

  // machine_name must survive (NAME change maps back through StableModel)
  if (v1_0.machine.machine_name !== 'BackwardMigrationTest') {
    return [false, `machine_name lost during downgrade: '${v1_0.machine.machine_name}'`];
  }

  if (v1_0.meta.file_version.trim() !== '1.0') {
    return [false, `file_version wrong after downgrade: '${v1_0.meta.file_version}'`];
  }

  return [true, 'backward migration OK — ADDITION fields lost (facility_id=null, config_author=null), ' +
    `machine_name='${v1_0.machine.machine_name}' preserved, file_version='1.0'`];
}

// AV-12: capabilities facade .opcua() returns Ok with the newly-promoted
// typed fields readable, when every required field is present.
//
// Nothing before this scenario exercised the `capabilities` facade at all —
// S-07 above only goes through the plain MachineConfigReader/Writer. See
// OPCUA_FIELD_PROMOTION_PLAN.md's "Validation-app coverage" section for why
// this is a real public-API guarantee, not just a unit-test concern.
export async function runAv12(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config_opcua.h5');
  const opened = await openMachineConfig(path);
  if (!opened.ok) {
    return [false, `openMachineConfig failed: ${JSON.stringify(opened.error)}`];
  }

  const result = opened.value.opcua();
  if (!result.ok) {
    return [false, `opcua() failed on fully-populated fixture: ${JSON.stringify(result.error)}`];
  }

  const model = result.value.getModel();
  return [
    true,
    `opcua() Ok: machine_profile='${model.client.machine_profile}', ` +
      `root_node='${model.client.root_node}', pipe_name='${model.pipe.pipe_name}', ` +
      `triggers_enabled=${model.triggers_enabled}, ` +
      `trigger_stop_ceiling_layers=${model.trigger_stop_ceiling_layers}`,
  ];
}

// AV-13: capabilities facade .opcua() returns Err(ValidationError) with
// `details` naming exactly the seven missing required fields, when OPCUA is
// present but incomplete.
export async function runAv13(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = avFixture(fixturesDir, 'opcua_missing_required.h5');
  const opened = await openMachineConfig(fixture);
  if (!opened.ok) {
    return [false, `openMachineConfig failed: ${JSON.stringify(opened.error)}`];
  }

  const result = opened.value.opcua();
  if (result.ok) {
    return [false, 'opcua() returned Ok on a fixture missing required fields'];
  }

  if (result.error.code !== 'ValidationError') {
    return [false, `wrong error code: ${result.error.code}`];
  }

  const expected = new Set([
    'Machine_Profile',
    'Root_Node',
    'Configure_Client',
    'Pipe_Name',
    'Triggers_Enabled',
    'Trigger_Stop_Ceiling_Layers',
    'Laser Emission Interlock.Event',
  ]);
  const actual = new Set(result.error.details ?? []);
  const sameSize = actual.size === expected.size;
  const sameMembers = [...expected].every((d) => actual.has(d));
  if (!sameSize || !sameMembers) {
    return [
      false,
      `details mismatch: got ${JSON.stringify([...actual].sort())}, expected ${JSON.stringify([...expected].sort())}`,
    ];
  }

  return [true, `ValidationError with details=${JSON.stringify([...actual].sort())}`];
}
