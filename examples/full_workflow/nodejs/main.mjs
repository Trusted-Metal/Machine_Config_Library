// Machine Config Library — Node.js Full Workflow: Calibration Adjustment
// =======================================================================
// Run from the repo root (build the library first if nodejs/dist/ is missing):
//
//   cd nodejs && npm run build && cd ..
//   node examples/full_workflow/nodejs/main.mjs
//
// Scenario: a field calibration measured new scanner-head positions for both
// optical trains.  Load the current machine config, apply the updated offsets,
// write the modified config to a new file, and verify the changes persisted
// alongside the binary correction data.

import { existsSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, basename } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, '..', '..', '..');
const FIXTURE   = join(REPO_ROOT, 'fixtures', 'reference_config.h5');

if (!existsSync(FIXTURE)) {
  console.error(`Fixture not found: ${FIXTURE}`);
  console.error('Run from the repo root or ensure fixtures/ is present.');
  process.exit(1);
}

const distIndex = join(REPO_ROOT, 'nodejs', 'dist', 'index.js');
if (!existsSync(distIndex)) {
  console.error(`Build output not found: ${distIndex}`);
  console.error('Run "npm run build" inside nodejs/ first.');
  process.exit(1);
}
const { MachineConfigReader, MachineConfigWriter } = await import(pathToFileURL(distIndex).href);

async function run() {
  // -------------------------------------------------------------------------
  // 1. Print pre-calibration summary
  // -------------------------------------------------------------------------
  const reader = new MachineConfigReader(FIXTURE);
  const config = await reader.parse();

  console.log('=== Full Workflow: Calibration Adjustment ===\n');
  console.log(`Machine : ${config.meta.machine_name}`);
  console.log(`Trains  : ${config.optical_trains.length}`);
  console.log();
  console.log('Before calibration:');
  for (let i = 0; i < config.optical_trains.length; i++) {
    const s  = config.optical_trains[i].scanner;
    const cd = await reader.getCorrectionData(i);
    console.log(`  Train ${i + 1}  offset x=${s.scan_head_offset_x}, y=${s.scan_head_offset_y}`);
    console.log(`           correction grid [${cd.shape.join(', ')}]`);
  }
  console.log();

  // -------------------------------------------------------------------------
  // 2. Apply new scanner offsets (post-calibration values)
  // -------------------------------------------------------------------------
  const newOffsets = [[-91.5, 24.0], [91.5, -24.0]];
  for (let i = 0; i < newOffsets.length; i++) {
    config.optical_trains[i].scanner.scan_head_offset_x = newOffsets[i][0];
    config.optical_trains[i].scanner.scan_head_offset_y = newOffsets[i][1];
  }

  // -------------------------------------------------------------------------
  // 3. Write updated config
  // -------------------------------------------------------------------------
  const outPath = join(tmpdir(), `machine_config_full_workflow_${randomUUID()}.h5`);
  await new MachineConfigWriter(config).write(outPath);
  console.log(`Written to : ${basename(outPath)}\n`);

  // -------------------------------------------------------------------------
  // 4. Read back and verify
  // -------------------------------------------------------------------------
  const reader2 = new MachineConfigReader(outPath);
  const updated = await reader2.parse();
  const failures = [];

  for (let i = 0; i < newOffsets.length; i++) {
    const [ex, ey] = newOffsets[i];
    const got_x    = updated.optical_trains[i].scanner.scan_head_offset_x;
    const got_y    = updated.optical_trains[i].scanner.scan_head_offset_y;
    if (got_x !== ex) failures.push(`  train${i + 1} offset_x: expected ${ex}, got ${got_x}`);
    if (got_y !== ey) failures.push(`  train${i + 1} offset_y: expected ${ey}, got ${got_y}`);
    const cd = await reader2.getCorrectionData(i);
    if (cd.shape[0] !== 257 || cd.shape[1] !== 257 || cd.shape[2] !== 2) {
      failures.push(`  train${i + 1} correction shape: expected [257,257,2], got [${cd.shape}]`);
    }
  }

  if (existsSync(outPath)) unlinkSync(outPath);

  console.log('After calibration:');
  for (let i = 0; i < updated.optical_trains.length; i++) {
    const s = updated.optical_trains[i].scanner;
    console.log(`  Train ${i + 1}  offset x=${s.scan_head_offset_x}, y=${s.scan_head_offset_y}`);
  }
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
