// Machine Config Library — Node.js Quickstart
// ============================================
// Run from the repo root (build the library first if nodejs/dist/ is missing):
//
//   cd nodejs && npm run build && cd ..
//   node examples/quickstart/nodejs/main.mjs
//
// Opens examples/dummy_2train.h5 via the stable model facade.

import { existsSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, basename } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, '..', '..', '..');
const DUMMY = join(REPO_ROOT, 'examples', 'dummy_2train.h5');

if (!existsSync(DUMMY)) {
  console.error(`Dummy file not found: ${DUMMY}`);
  console.error('Run: python examples/generate_dummy.py');
  process.exit(1);
}

const distIndex = join(REPO_ROOT, 'nodejs', 'dist', 'index.js');
if (!existsSync(distIndex)) {
  console.error(`Build output not found: ${distIndex}`);
  console.error('Run "npm run build" inside nodejs/ first.');
  process.exit(1);
}
const {
  openMachineConfig,
  MachineConfigReader,
  SetMode,
} = await import(pathToFileURL(distIndex).href);

async function run() {
  const opened = await openMachineConfig(DUMMY);
  if (!opened.ok) {
    console.error(`open failed: ${opened.error.code} — ${opened.error.message}`);
    process.exit(1);
  }
  const file = opened.value;

  console.log('=== Machine Config Quickstart ===\n');
  console.log(`File version   : ${file.fileVersion()}`);
  const meta = file.meta().getModel();
  console.log(`Machine name   : ${meta.machine_name}`);
  console.log(`Optical trains : ${file.opticalTrains().length}`);

  let i = 0;
  for (const train of file.opticalTrains()) {
    const scanner = train.getScanner();
    console.log(
      `  Train ${i}  wd=${scanner.working_distance} ${scanner.working_distance_unit ?? ''}  ` +
        `offset x=${scanner.scan_head_offset_x}, y=${scanner.scan_head_offset_y}`,
    );
    const oc = train.optionalComponents();
    if (oc == null) {
      console.log('           optionalComponents: none');
    } else {
      const cb = oc.clearbox();
      console.log(`           clearbox: ${cb.ok ? 'present' : cb.error.code}`);
    }
    i += 1;
  }

  const train0 = file.opticalTrain(0);
  if (!train0.ok) {
    console.error(train0.error.message);
    process.exit(1);
  }
  const scanner = train0.value.getScanner();

  const reader = new MachineConfigReader(DUMMY);
  const correction = await reader.getCorrectionData(0);
  console.log(`Correction grid: [${correction.shape.join(', ')}]   (train 0)`);

  console.log();
  const tmpPath = join(tmpdir(), `machine_config_quickstart_${randomUUID()}.h5`);
  const setR = train0.value.setScanner(scanner, SetMode.Merge);
  if (!setR.ok) {
    console.error(`setScanner failed: ${setR.error.message}`);
    process.exit(1);
  }
  const saved = await file.save(tmpPath);
  if (!saved.ok) {
    console.error(`save failed: ${saved.error.message}`);
    process.exit(1);
  }
  console.log(`Written to     : ${basename(tmpPath)}`);

  const again = await openMachineConfig(tmpPath);
  if (!again.ok) {
    console.error(again.error.message);
    process.exit(1);
  }
  const name2 = again.value.meta().getModel().machine_name;
  const n2 = again.value.opticalTrains().length;
  const t2 = again.value.opticalTrain(0);
  const wd2 = t2.ok ? t2.value.getScanner().working_distance : null;

  const failures = [];
  if (name2 !== meta.machine_name) {
    failures.push('  machine_name mismatch after round-trip');
  }
  if (n2 !== file.opticalTrains().length) {
    failures.push(`  train_count: expected ${file.opticalTrains().length}, got ${n2}`);
  }
  if (wd2 !== scanner.working_distance) {
    failures.push(`  working_distance: expected ${scanner.working_distance}, got ${wd2}`);
  }

  file.close();
  again.value.close();
  if (existsSync(tmpPath)) unlinkSync(tmpPath);

  console.log();
  if (failures.length > 0) {
    console.log('FAIL');
    for (const msg of failures) console.log(msg);
    process.exit(1);
  } else {
    console.log('PASS');
  }
}

try {
  await run();
} catch (e) {
  console.error(`error: ${e?.message ?? e}`);
  process.exit(1);
}
