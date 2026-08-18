import {
  runS01, runS02, runS03, runS04, runS05, runS06, runS07, runS08, runS09,
  runAv01, runAv02, runAv03, runAv04, runAv05, runAv06, runAv07, runAv08,
  runAv09, runAv10, runAv11,
} from './scenarios.mts';

type RunFn = (fixturesDir: string, realDir: string) => Promise<[boolean, string]>;

const SCENARIOS: [string, RunFn][] = [
  ['S-01', runS01], ['S-02', runS02], ['S-03', runS03], ['S-04', runS04],
  ['S-05', runS05], ['S-06', runS06], ['S-07', runS07], ['S-08', runS08],
  ['S-09', runS09],
  ['AV-01', runAv01], ['AV-02', runAv02], ['AV-03', runAv03], ['AV-04', runAv04],
  ['AV-05', runAv05], ['AV-06', runAv06], ['AV-07', runAv07], ['AV-08', runAv08],
  ['AV-09', runAv09], ['AV-10', runAv10], ['AV-11', runAv11],
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
