import * as s01 from './scenarios/s01_read_scalars.mts';
import * as s02 from './scenarios/s02_read_binary.mts';
import * as s03 from './scenarios/s03_read_real.mts';
import * as s04 from './scenarios/s04_write_modify.mts';
import * as s05 from './scenarios/s05_binary_roundtrip.mts';
import * as s06 from './scenarios/s06_builder.mts';
import * as s07 from './scenarios/s07_opcua.mts';
import * as s08 from './scenarios/s08_drastic_change.mts';
import * as s09 from './scenarios/s09_type_exports.mts';
import * as av01 from './scenarios/av01_unknown_version.mts';
import * as av02 from './scenarios/av02_missing_version.mts';
import * as av03 from './scenarios/av03_future_version.mts';
import * as av04 from './scenarios/av04_missing_group.mts';
import * as av05 from './scenarios/av05_corrupt_scalar.mts';
import * as av06 from './scenarios/av06_whitespace_version.mts';
import * as av07 from './scenarios/av07_empty_version.mts';
import * as av08 from './scenarios/av08_version_fidelity.mts';
import * as av09 from './scenarios/av09_mock_adapter_isolation.mts';
import * as av10 from './scenarios/av10_forward_migration.mts';
import * as av11 from './scenarios/av11_backward_migration.mts';

type RunFn = (fixturesDir: string, realDir: string) => Promise<[boolean, string]>;

const SCENARIOS: [string, RunFn][] = [
  ['S-01', s01.run], ['S-02', s02.run], ['S-03', s03.run], ['S-04', s04.run],
  ['S-05', s05.run], ['S-06', s06.run], ['S-07', s07.run], ['S-08', s08.run],
  ['S-09', s09.run],
  ['AV-01', av01.run], ['AV-02', av02.run], ['AV-03', av03.run], ['AV-04', av04.run],
  ['AV-05', av05.run], ['AV-06', av06.run], ['AV-07', av07.run], ['AV-08', av08.run],
  ['AV-09', av09.run], ['AV-10', av10.run], ['AV-11', av11.run],
];

async function main(): Promise<void> {
  const [fixturesDir, realDir] = process.argv.slice(2);
  if (!fixturesDir || !realDir) {
    console.error('Usage: node main.mts <fixtures_dir> <real_dir>');
    process.exit(1);
  }

  let passed = 0, failed = 0;
  for (const [sid, runFn] of SCENARIOS) {
    let ok: boolean, msg: string;
    try {
      [ok, msg] = await runFn(fixturesDir, realDir);
    } catch (e) {
      ok = false;
      msg = `EXCEPTION: ${(e as Error).constructor.name}: ${(e as Error).message}`;
    }
    console.log(`${ok ? '[PASS]' : '[FAIL]'} ${sid}: ${msg}`);
    ok ? passed++ : failed++;
  }

  console.log(`\n${passed + failed} scenarios: ${passed} passed, ${failed} failed`);
  process.exit(failed === 0 ? 0 : 1);
}

main();
