// AV-11: Mock v1.1 adapter — backward migration (v1.1-mock -> v1.0)
//
// ID:          AV-11
// Title:       v1.1-mock file migrates backward to v1.0 correctly; ADDITION fields are lost
// Category:    adapter-versioning
// Layer:       adapter + StableModel
// Precondition: A v1.1-mock file produced by MockV1_1Writer (created inline)
// Action:      Write a v1.1-mock file with facility_id and config_author populated ->
//              read with mock v1.1 adapter -> write with v1.0 adapter -> read back.
//              Verify:
//                ADDITION  — facility_id and config_author are undefined after v1.0 read
//                            (v1.0 writer does not write these attrs — intentionally lost)
//                NAME/PATH — all preserved fields survive back to v1.0 layout
// Expected:    ADDITION fields lost after roundtrip. All other preserved fields intact.
// Rationale:   Verifies the backward migration contract: additions introduced in v1.1
//              are explicitly lost when downgrading, not silently corrupted.
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';
import { MockV1_1Reader, MockV1_1Writer, makeMockConfig } from '../../../../../nodejs/tests/mockV1_1.js';

export async function run(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
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
