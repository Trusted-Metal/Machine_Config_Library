/**
 * Adapter migration tests.
 *
 * Proves the StableModel contract holds across adapter version boundaries.
 * MockV1_1 (see `./mockV1_1.ts`) introduces all five change categories
 * relative to v1.0, exactly mirroring `python/tests/test_adapter_migration.py`
 * and `docs/migrations/mock_v1_0_to_v1_1.md`:
 *
 *   Addition  (x2): root attrs Facility_ID, Config_Author -> meta.facility_id/config_author
 *   Removal   (x2): Machine/ drops Gas_Flow_Direction, Recoat_Direction -> null
 *   Name      (x2): Machine/ "Machine_Name" -> "Machine_Label"
 *                   Scanner/ "Working_Distance" -> "Focal_Distance"
 *   Path      (x2): Build_Plate_Corner_Radius, Build_Plate_Z_Dimension
 *                   move from Machine/ to Machine/Dimensions/
 *   Name+path (x2): Build_Plate_X_Dimension -> Machine/Dimensions/ "Width"
 *                   Build_Plate_Y_Dimension -> Machine/Dimensions/ "Height"
 */
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import type { MachineConfig } from '../src/models.js';
import { Hdf5AdapterV1_0, type ReadOptions } from '../src/capabilities/v1_0/hdf5.js';
import { Hdf5WriterV1_0 } from '../src/capabilities/v1_0/writer.js';
import { MachineConfigReader, _READERS } from '../src/reader.js';
import { MachineConfigWriter, _WRITERS } from '../src/writer.js';
import { FILE_VERSION, MockV1_1Reader, MockV1_1Writer, makeMockConfig } from './mockV1_1.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE = join(__dirname, '../../fixtures/reference_config.h5');

function tempH5Path(prefix: string): string {
  return join(tmpdir(), `${prefix}_${randomUUID()}.h5`);
}

// ---------------------------------------------------------------------------
// Tests — adapter-level (bypass the dispatcher)
// ---------------------------------------------------------------------------

describe('adapter migration — mock v1.1 (all five change categories)', () => {
  it('reads all five change categories from a mock v1.1 file', async () => {
    const cfg = makeMockConfig({ facilityId: 'Lab-001', configAuthor: 'TestEngineer' });
    const p = tempH5Path('v1_1_read');
    await new MockV1_1Writer(cfg).write(p);
    const result = await new MockV1_1Reader(p).parse();

    // ADDITION (x2)
    expect(result.meta.facility_id).toBe('Lab-001');
    expect(result.meta.config_author).toBe('TestEngineer');
    // REMOVAL (x2)
    expect(result.machine.gas_flow_direction).toBeNull();
    expect(result.machine.recoat_direction).toBeNull();
    // NAME (x2)
    expect(result.machine.machine_name).toBe('MigrationTestMachine');
    expect(result.optical_trains[0].scanner.working_distance).toBe(cfg.optical_trains[0].scanner.working_distance);
    // PATH (x2)
    expect(result.machine.build_plate_z).toBe(cfg.machine.build_plate_z);
    expect(result.machine.build_plate_radius).toBe(cfg.machine.build_plate_radius);
    // NAME+PATH (x2)
    expect(result.machine.build_plate_x).toBe(250.0);
    expect(result.machine.build_plate_y).toBe(175.0);
  });

  it('survives write -> read -> write -> read (roundtrip)', async () => {
    const cfg = makeMockConfig({ facilityId: 'RoundtripLab', configAuthor: 'RoundtripEngineer' });
    const p1 = tempH5Path('v1_1_a');
    await new MockV1_1Writer(cfg).write(p1);
    const mid = await new MockV1_1Reader(p1).parse();
    const p2 = tempH5Path('v1_1_b');
    await new MockV1_1Writer(mid).write(p2);
    const result = await new MockV1_1Reader(p2).parse();

    expect(result.meta.facility_id).toBe('RoundtripLab');
    expect(result.meta.config_author).toBe('RoundtripEngineer');
    expect(result.machine.gas_flow_direction).toBeNull();
    expect(result.machine.recoat_direction).toBeNull();
    expect(result.machine.machine_name).toBe(cfg.machine.machine_name);
    expect(result.optical_trains[0].scanner.working_distance).toBe(cfg.optical_trains[0].scanner.working_distance);
    expect(result.machine.build_plate_z).toBe(cfg.machine.build_plate_z);
    expect(result.machine.build_plate_radius).toBe(cfg.machine.build_plate_radius);
    expect(result.machine.build_plate_x).toBe(cfg.machine.build_plate_x);
    expect(result.machine.build_plate_y).toBe(cfg.machine.build_plate_y);
  });

  it('migrates a real v1.0 fixture forward to mock v1.1 — surviving fields preserved', async () => {
    const source = await new Hdf5AdapterV1_0(REFERENCE).parse();
    const forMock = { ...source, meta: { ...source.meta, file_version: FILE_VERSION } };
    const out = tempH5Path('migrated_v1_1');
    await new MockV1_1Writer(forMock).write(out);
    const result = await new MockV1_1Reader(out).parse();

    // NAME: machine_name survives both the read and write name remapping
    expect(result.machine.machine_name).toBe(source.machine.machine_name);
    // NAME+PATH: x and y survive via the StableModel intermediary
    expect(result.machine.build_plate_x).toBe(source.machine.build_plate_x);
    expect(result.machine.build_plate_y).toBe(source.machine.build_plate_y);
    // PATH: z and radius survive
    expect(result.machine.build_plate_z).toBe(source.machine.build_plate_z);
    expect(result.machine.build_plate_radius).toBe(source.machine.build_plate_radius);
    // NAME: scanner working_distance survives
    expect(result.optical_trains[0].scanner.working_distance).toBe(source.optical_trains[0].scanner.working_distance);
    // REMOVAL: always null regardless of what the v1.0 source contained
    expect(result.machine.gas_flow_direction).toBeNull();
    expect(result.machine.recoat_direction).toBeNull();
    // ADDITION: no v1.0 source -> fields are null after forward migration
    expect(result.meta.facility_id).toBeNull();
    expect(result.meta.config_author).toBeNull();
  });

  it('migrates a mock v1.1 file backward to v1.0 — ADDITION fields lost', async () => {
    const cfg = makeMockConfig({ facilityId: 'Lab-V11', configAuthor: 'MigrationBot' });
    const v1_1Path = tempH5Path('source_v1_1');
    await new MockV1_1Writer(cfg).write(v1_1Path);
    const v1_1Config = await new MockV1_1Reader(v1_1Path).parse();

    const forV1 = { ...v1_1Config, meta: { ...v1_1Config.meta, file_version: '1.0' } };
    const v1Out = tempH5Path('migrated_v1');
    await new Hdf5WriterV1_0(forV1).write(v1Out);
    const result = await new Hdf5AdapterV1_0(v1Out).parse();

    expect(result.machine.machine_name).toBe(cfg.machine.machine_name);
    expect(result.machine.build_plate_x).toBe(cfg.machine.build_plate_x);
    expect(result.machine.build_plate_y).toBe(cfg.machine.build_plate_y);
    expect(result.machine.build_plate_z).toBe(cfg.machine.build_plate_z);
    expect(result.machine.build_plate_radius).toBe(cfg.machine.build_plate_radius);
    expect(result.optical_trains[0].scanner.working_distance).toBe(cfg.optical_trains[0].scanner.working_distance);

    // REMOVAL: absent in mock v1.1 -> remain null after roundtrip through v1.0
    expect(result.machine.gas_flow_direction).toBeNull();
    expect(result.machine.recoat_direction).toBeNull();
    // ADDITION: typed v1.1 fields are lost on downgrade (v1.0 writer does not write them)
    expect(result.meta.facility_id).toBeUndefined();
    expect(result.meta.config_author).toBeUndefined();
  });

  it('leaves the existing v1.0 read path completely undisturbed', async () => {
    const config = await new Hdf5AdapterV1_0(REFERENCE).parse();
    expect(config.meta.file_version).toBe('1.0');
    expect(config.optical_trains.length).toBeGreaterThan(0);
    expect(config.machine).toBeDefined();
    expect(config.machine.build_plate_x).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Tests — dispatcher-level (full public API via injected _READERS/_WRITERS)
// ---------------------------------------------------------------------------

describe('adapter migration — public API dispatch', () => {
  beforeEach(() => {
    _READERS[FILE_VERSION] = MockV1_1Reader as unknown as (typeof _READERS)[string];
    _WRITERS[FILE_VERSION] = MockV1_1Writer as unknown as (typeof _WRITERS)[string];
  });

  afterEach(() => {
    delete _READERS[FILE_VERSION];
    delete _WRITERS[FILE_VERSION];
  });

  it('routes a mock v1.1 file through the real MachineConfigReader/Writer, then migrates it to v1.0', async () => {
    const cfg = makeMockConfig({ machineName: 'DispatcherTest', facilityId: 'Dispatch-Lab' });
    const v1_1File = tempH5Path('dispatcher_v1_1');
    await new MockV1_1Writer(cfg).write(v1_1File);

    // Public API read — dispatcher peeks "1.1-mock" and routes to MockV1_1Reader
    const result = await new MachineConfigReader(v1_1File).parse();
    expect(result.meta.file_version).toBe(FILE_VERSION);
    expect(result.meta.facility_id).toBe('Dispatch-Lab');
    expect(result.machine.machine_name).toBe(cfg.machine.machine_name);
    expect(result.machine.build_plate_x).toBe(cfg.machine.build_plate_x);

    // Public API write to v1.0 — dispatcher sees file_version "1.0" and routes to Hdf5WriterV1_0
    const v1Out = tempH5Path('dispatcher_migrated_v1');
    await new MachineConfigWriter({ ...result, meta: { ...result.meta, file_version: '1.0' } }).write(v1Out);
    const final = await new MachineConfigReader(v1Out).parse();
    expect(final.meta.file_version).toBe('1.0');
    expect(final.machine.machine_name).toBe(cfg.machine.machine_name);
    expect(final.machine.build_plate_x).toBe(cfg.machine.build_plate_x);
  });

  it('routes a real v1.0 file migrated to mock v1.1 through the public API', async () => {
    const source = await new Hdf5AdapterV1_0(REFERENCE).parse();

    // Public API write — dispatcher sees file_version "1.1-mock" and routes to MockV1_1Writer
    const v1_1Out = tempH5Path('dispatcher_v1_1_out');
    await new MachineConfigWriter({ ...source, meta: { ...source.meta, file_version: FILE_VERSION } }).write(v1_1Out);

    // Public API read — dispatcher peeks "1.1-mock" and routes to MockV1_1Reader
    const result = await new MachineConfigReader(v1_1Out).parse();
    expect(result.meta.file_version).toBe(FILE_VERSION);
    expect(result.machine.machine_name).toBe(source.machine.machine_name);
    expect(result.machine.build_plate_x).toBe(source.machine.build_plate_x);
    expect(result.optical_trains[0].scanner.working_distance).toBe(source.optical_trains[0].scanner.working_distance);
    expect(result.machine.gas_flow_direction).toBeNull(); // removed in v1.1-mock
    expect(result.machine.recoat_direction).toBeNull();
  });

  it('satisfies the same structural shape MachineConfigReader/Writer expect', () => {
    // Compile-time structural check: if this assigns without error, MockV1_1Reader/Writer
    // are structurally compatible with what the dispatch tables require.
    const readerCtor: new (path: string) => { parse: (o?: ReadOptions) => Promise<MachineConfig> } = MockV1_1Reader;
    const writerCtor: new (config: MachineConfig) => { write: (path: string) => Promise<void> } = MockV1_1Writer;
    expect(readerCtor).toBe(MockV1_1Reader);
    expect(writerCtor).toBe(MockV1_1Writer);
  });
});
