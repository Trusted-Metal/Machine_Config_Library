// AV-10: Mock v1.1 adapter — forward migration (v1.0 -> v1.1-mock)
//
// ID:          AV-10
// Title:       v1.0 fixture migrates forward to mock v1.1 correctly across all change categories
// Category:    adapter-versioning
// Layer:       adapter + StableModel
// Precondition: fixtures/reference_config.h5 (v1.0)
// Action:      Read with v1.0 adapter -> write with mock v1.1 adapter -> read back.
//              Verify all five change categories from docs/migrations/mock_v1_0_to_v1_1.md:
//                ADDITION  — facility_id and config_author are null (no v1.0 source)
//                REMOVAL   — gas_flow_direction and recoat_direction are null (dropped in v1.1)
//                NAME      — machine_name and working_distance values preserved
//                PATH      — build_plate_z and build_plate_radius values preserved
//                NAME+PATH — build_plate_x and build_plate_y values preserved
// Expected:    Preserved fields identical before and after. Lossy fields are null.
// Rationale:   Verifies the StableModel is the correct handoff point and that each
//              change category behaves as documented in the manifest.
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';
import { MockV1_1Reader, MockV1_1Writer } from '../../../../../nodejs/tests/mockV1_1.js';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
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
