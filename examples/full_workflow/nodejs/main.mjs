// Machine Config Library — Node.js Full Workflow: Calibration Adjustment
// =======================================================================
// Run from the repo root (build the library first if nodejs/dist/ is missing):
//
//   cd nodejs && npm run build && cd ..
//   node examples/full_workflow/nodejs/main.mjs
//
// Load examples/dummy_2train.h5, apply new scanner offsets via setScanner(Merge).

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
    console.error(`open failed: ${opened.error.message}`);
    process.exit(1);
  }
  const file = opened.value;

  console.log('=== Full Workflow: Calibration Adjustment ===\n');
  console.log(`Machine : ${file.meta().getModel().machine_name}`);
  console.log(`Trains  : ${file.opticalTrains().length}`);
  console.log();
  console.log('Before calibration:');
  const reader = new MachineConfigReader(DUMMY);
  let i = 0;
  for (const train of file.opticalTrains()) {
    const s = train.getScanner();
    const cd = await reader.getCorrectionData(i);
    console.log(`  Train ${i + 1}  offset x=${s.scan_head_offset_x}, y=${s.scan_head_offset_y}`);
    console.log(`           correction grid [${cd.shape.join(', ')}]`);
    i += 1;
  }
  console.log();

  const newOffsets = [[-91.5, 24.0], [91.5, -24.0]];
  for (let t = 0; t < newOffsets.length; t++) {
    const train = file.opticalTrain(t);
    if (!train.ok) {
      console.error(train.error.message);
      process.exit(1);
    }
    const scanner = train.value.getScanner();
    scanner.scan_head_offset_x = newOffsets[t][0];
    scanner.scan_head_offset_y = newOffsets[t][1];
    const setR = train.value.setScanner(scanner, SetMode.Merge);
    if (!setR.ok) {
      console.error(setR.error.message);
      process.exit(1);
    }
  }

  const outPath = join(tmpdir(), `machine_config_full_workflow_${randomUUID()}.h5`);
  const saved = await file.save(outPath);
  if (!saved.ok) {
    console.error(`save failed: ${saved.error.message}`);
    process.exit(1);
  }
  file.close();
  console.log(`Written to : ${basename(outPath)}\n`);

  const again = await openMachineConfig(outPath);
  if (!again.ok) {
    console.error(again.error.message);
    process.exit(1);
  }
  const updated = again.value;
  const reader2 = new MachineConfigReader(outPath);
  const failures = [];

  for (let t = 0; t < newOffsets.length; t++) {
    const [ex, ey] = newOffsets[t];
    const train = updated.opticalTrain(t);
    if (!train.ok) {
      failures.push(`  train${t + 1}: ${train.error.message}`);
      continue;
    }
    const s = train.value.getScanner();
    if (s.scan_head_offset_x !== ex) {
      failures.push(`  train${t + 1} offset_x: expected ${ex}, got ${s.scan_head_offset_x}`);
    }
    if (s.scan_head_offset_y !== ey) {
      failures.push(`  train${t + 1} offset_y: expected ${ey}, got ${s.scan_head_offset_y}`);
    }
    const cd = await reader2.getCorrectionData(t);
    if (cd.shape[0] !== 257 || cd.shape[1] !== 257 || cd.shape[2] !== 2) {
      failures.push(`  train${t + 1} correction shape: expected [257,257,2], got [${cd.shape}]`);
    }
  }

  console.log('After calibration:');
  i = 0;
  for (const train of updated.opticalTrains()) {
    const s = train.getScanner();
    console.log(`  Train ${i + 1}  offset x=${s.scan_head_offset_x}, y=${s.scan_head_offset_y}`);
    i += 1;
  }
  console.log();

  updated.close();
  if (existsSync(outPath)) unlinkSync(outPath);

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
