// AV-09: Mock v1.1 adapter — adding a new adapter leaves the v1.0 adapter untouched
//
// ID:          AV-09
// Title:       Adding a mock v1.1 adapter leaves the v1.0 adapter and all existing behaviour unchanged
// Category:    adapter-versioning
// Layer:       adapter isolation
// Precondition: fixtures/reference_config.h5
// Action:      Import MockV1_1Reader/Writer alongside the real v1.0 reader.
//              Read the reference fixture with the real v1.0 reader and verify correct output.
// Expected:    v1.0 reader still returns correct values — importing the mock v1.1 adapter
//              has no side-effect on the v1.0 adapter's behaviour.
// Rationale:   The adapter pattern's primary promise is that adding a new version is
//              purely additive — no existing adapter code is modified.
//
// The mock lives in nodejs/tests/mockV1_1.ts (never packaged) — imported directly from
// the source tree, mirroring Python's sys.path trick in the equivalent scenario. This only
// works because this scenario file lives in-repo, at a fixed relative offset from nodejs/tests/.
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';
// eslint-disable-next-line @typescript-eslint/no-unused-vars -- imported to prove co-existence, see Action above
import { MockV1_1Reader, MockV1_1Writer } from '../../../../../nodejs/tests/mockV1_1.js';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
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
