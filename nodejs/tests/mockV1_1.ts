/**
 * Mock v1.1 adapter — test artifact only, exercises the change-category
 * architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
 * change. See `docs/migrations/mock_v1_0_to_v1_1.md` for the full manifest.
 *
 * Deliberately *not* a `.test.ts` file: `adapterMigration.test.ts` and
 * `scratch/inspectMigration.mts` both import from here. Keeping the mock
 * classes out of a Vitest test file means a plain script can import them
 * without also pulling in `describe`/`it` calls that only make sense inside
 * Vitest's own test runner.
 *
 * Design: the reader/writer *delegate* to the real, public v1.0 adapter for
 * the whole file, then patch exactly the 10 documented differences, rather
 * than reimplementing or reaching into v1.0's private parsing internals.
 * "Unchanged" subcomponents (light_source, collimator, scanner_card,
 * clearbox, sfcf, opcua) are never re-tested here — they're already covered
 * by the 143 tests exercising the real v1.0 adapter, and duplicating that
 * coverage would only create a second copy that could drift from the real
 * implementation. Verified empirically (see chat log) that h5wasm supports
 * the required operations (delete_attribute, create_group on an
 * already-written file) before committing to this design.
 */
import { resolve } from 'node:path';
import * as h5wasm from 'h5wasm/node';

import { MockConfigBuilder } from '../src/builder.js';
import type { MachineConfig } from '../src/models.js';
import { Hdf5AdapterV1_0, type ReadOptions } from '../src/capabilities/v1_0/hdf5.js';
import { Hdf5WriterV1_0 } from '../src/capabilities/v1_0/writer.js';

// ---------------------------------------------------------------------------
// MockV1_1Layout — on-disk constants that differ from v1.0
// ---------------------------------------------------------------------------

export const FILE_VERSION = '1.1-mock';
export const ATTR_FACILITY_ID = 'Facility_ID';
export const ATTR_CONFIG_AUTHOR = 'Config_Author';
export const ATTR_MACHINE_LABEL = 'Machine_Label';
export const ATTR_FOCAL_DISTANCE = 'Focal_Distance';
export const DIMENSIONS_PATH = 'Machine/Dimensions';
export const ATTR_BP_WIDTH = 'Width';
export const ATTR_BP_HEIGHT = 'Height';

function normPath(p: string): string {
  return resolve(p).replace(/\\/g, '/');
}

/** Read a scalar attribute value directly off a raw h5wasm attrs record, or null if absent/empty. */
function readRaw(attrs: Record<string, h5wasm.Attribute>, key: string): string | number | null {
  const v = attrs[key]?.value;
  if (v == null) return null;
  if (typeof v === 'string' && !v.trim()) return null;
  return v as string | number;
}

function readOptionalFloat(attrs: Record<string, h5wasm.Attribute>, key: string): number | null {
  const v = readRaw(attrs, key);
  if (v == null) return null;
  const n = Number(v);
  return Number.isNaN(n) ? null : n;
}

function readOptionalStr(attrs: Record<string, h5wasm.Attribute>, key: string): string | null {
  const v = readRaw(attrs, key);
  return v == null ? null : String(v).trim() || null;
}

/**
 * `Attribute.value` is typed to allow `bigint` (for 64-bit integer HDF5
 * types); `create_attribute` doesn't accept it directly. None of the values
 * this mock copies are ever actually bigint (they're the same string/float
 * attrs the real writer just wrote), so narrowing to Number is safe here.
 */
function sanitizeAttrValue(v: h5wasm.OutputData | null): string | number {
  if (v == null) return '';
  if (typeof v === 'bigint') return Number(v);
  if (typeof v === 'string' || typeof v === 'number') return v;
  return String(v);
}

// ---------------------------------------------------------------------------
// MockV1_1Reader — delegates to the real v1.0 parser, then patches the delta
// ---------------------------------------------------------------------------

export class MockV1_1Reader {
  constructor(private readonly path: string) {}

  async parse(options: ReadOptions = {}): Promise<MachineConfig> {
    // Step 1: the real v1.0 parser correctly reads every subcomponent this
    // mock doesn't change. Fields it can't find (because they were renamed,
    // moved, or removed) come back null/empty — never a throw, since none
    // of the 10 changes remove or rename a *group*, only attributes within
    // one, or the presence of an attribute (see docs/migrations/mock_v1_0_to_v1_1.md).
    const base = await new Hdf5AdapterV1_0(this.path).parse(options);

    // Step 2: patch exactly the 10 documented differences by reading their
    // real, mock-v1.1 locations directly.
    await h5wasm.ready;
    const f = new h5wasm.File(normPath(this.path), 'r');
    try {
      const machineEnt = f.get('Machine');
      const machine = machineEnt instanceof h5wasm.Group ? machineEnt : null;
      const dimsEnt = f.get(DIMENSIONS_PATH);
      const dims = dimsEnt instanceof h5wasm.Group ? dimsEnt : null;

      const facilityId = readOptionalStr(f.attrs, ATTR_FACILITY_ID);
      const configAuthor = readOptionalStr(f.attrs, ATTR_CONFIG_AUTHOR);
      const machineLabel = machine ? readOptionalStr(machine.attrs, ATTR_MACHINE_LABEL) ?? '' : '';
      const buildPlateX = dims ? readOptionalFloat(dims.attrs, ATTR_BP_WIDTH) : null;
      const buildPlateY = dims ? readOptionalFloat(dims.attrs, ATTR_BP_HEIGHT) : null;
      const buildPlateZ = dims ? readOptionalFloat(dims.attrs, 'Build_Plate_Z_Dimension') : null;
      const buildPlateRadius = dims ? readOptionalFloat(dims.attrs, 'Build_Plate_Corner_Radius') : null;

      const optical_trains = base.optical_trains.map((train) => {
        const scannerEnt = f.get(`Machine/Optical_Trains/${train.train_id}/Scanner`);
        const focal = scannerEnt instanceof h5wasm.Group
          ? readOptionalFloat(scannerEnt.attrs, ATTR_FOCAL_DISTANCE)
          : null;
        return { ...train, scanner: { ...train.scanner, working_distance: focal } };
      });

      // The base parser doesn't know Facility_ID/Config_Author are typed
      // fields, so it swept them into meta.extra as unknown attrs. Strip them
      // out there before setting the typed fields, or they'd exist in both
      // places — and get written twice, colliding, on the next write.
      const { [ATTR_FACILITY_ID]: _fid, [ATTR_CONFIG_AUTHOR]: _cauth, ...restExtra } = base.meta.extra;

      return {
        ...base,
        meta: { ...base.meta, extra: restExtra, facility_id: facilityId, config_author: configAuthor },
        machine: {
          ...base.machine,
          machine_name: machineLabel,
          build_plate_x: buildPlateX,
          build_plate_y: buildPlateY,
          build_plate_z: buildPlateZ,
          build_plate_radius: buildPlateRadius,
          // gas_flow_direction / recoat_direction: already null from the base
          // parse (the attrs are genuinely absent) — no patch needed.
        },
        optical_trains,
      };
    } finally {
      f.close();
    }
  }
}

// ---------------------------------------------------------------------------
// MockV1_1Writer — writes a real v1.0-shaped file, then patches the delta
// ---------------------------------------------------------------------------

export class MockV1_1Writer {
  constructor(private readonly config: MachineConfig) {}

  async write(outputPath: string): Promise<void> {
    // Step 1: the real v1.0 writer correctly writes every subcomponent this
    // mock doesn't change, plus File_Version itself (already "1.1-mock" on
    // the input config — the writer just persists whatever string is there).
    await new Hdf5WriterV1_0(this.config).write(outputPath);

    // Step 2: patch exactly the 10 documented differences in place.
    await h5wasm.ready;
    const f = new h5wasm.File(normPath(outputPath), 'a');
    try {
      const machine = f.get('Machine') as h5wasm.Group;

      // ADDITION (x2)
      f.create_attribute(ATTR_FACILITY_ID, this.config.meta.facility_id ?? '', [], 'S');
      f.create_attribute(ATTR_CONFIG_AUTHOR, this.config.meta.config_author ?? '', [], 'S');

      // REMOVAL (x2)
      machine.delete_attribute('Gas_Flow_Direction');
      machine.delete_attribute('Recoat_Direction');

      // NAME (Machine_Name -> Machine_Label) — preserve dtype/shape from the
      // attribute the real writer just wrote, rather than re-guessing them.
      this.renameAttr(machine, 'Machine_Name', ATTR_MACHINE_LABEL);

      // PATH / NAME+PATH: build-plate values move into Machine/Dimensions/.
      // Unit attrs are unaffected — they stay on Machine/ per the manifest.
      const dims = machine.create_group('Dimensions');
      this.moveAttr(machine, dims, 'Build_Plate_X_Dimension', ATTR_BP_WIDTH);
      this.moveAttr(machine, dims, 'Build_Plate_Y_Dimension', ATTR_BP_HEIGHT);
      this.moveAttr(machine, dims, 'Build_Plate_Z_Dimension', 'Build_Plate_Z_Dimension');
      this.moveAttr(machine, dims, 'Build_Plate_Corner_Radius', 'Build_Plate_Corner_Radius');

      // NAME (Working_Distance -> Focal_Distance), once per optical train.
      const trainsGrp = f.get('Machine/Optical_Trains');
      if (trainsGrp instanceof h5wasm.Group) {
        for (const trainId of trainsGrp.keys()) {
          const scannerEnt = f.get(`Machine/Optical_Trains/${trainId}/Scanner`);
          if (scannerEnt instanceof h5wasm.Group) {
            this.renameAttr(scannerEnt, 'Working_Distance', ATTR_FOCAL_DISTANCE);
          }
        }
      }
    } finally {
      f.close();
    }
  }

  private renameAttr(grp: h5wasm.Group, oldKey: string, newKey: string): void {
    const attr = grp.attrs[oldKey];
    if (attr == null) return;
    grp.create_attribute(newKey, sanitizeAttrValue(attr.value), attr.shape, attr.dtype);
    grp.delete_attribute(oldKey);
  }

  private moveAttr(from: h5wasm.Group, to: h5wasm.Group, oldKey: string, newKey: string): void {
    const attr = from.attrs[oldKey];
    if (attr == null) return;
    to.create_attribute(newKey, sanitizeAttrValue(attr.value), attr.shape, attr.dtype);
    from.delete_attribute(oldKey);
  }
}

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's `_make_config()`
// ---------------------------------------------------------------------------

export function makeMockConfig(opts: {
  machineName?: string;
  facilityId?: string | null;
  configAuthor?: string | null;
} = {}): MachineConfig {
  const cfg = new MockConfigBuilder({
    nLasers: 1,
    buildPlateX: 250.0,
    buildPlateY: 175.0, // distinct from x so name+path assertions are unambiguous
    fileVersion: FILE_VERSION,
    machineName: opts.machineName ?? 'MigrationTestMachine',
  }).build();
  return {
    ...cfg,
    meta: {
      ...cfg.meta,
      facility_id: opts.facilityId ?? null,
      config_author: opts.configAuthor ?? null,
    },
    machine: {
      ...cfg.machine,
      gas_flow_direction: null, // absent in v1.1-mock by design
      recoat_direction: null,
    },
  };
}
