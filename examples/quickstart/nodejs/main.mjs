// Machine Config Library — Node.js Quickstart
// ============================================
// Run from the repo root (build the library first if nodejs/dist/ is missing):
//
//   cd nodejs && npm run build && cd ..
//   node examples/quickstart/nodejs/main.mjs
//
// Demonstrates the six essential operations:
//   1. Open an HDF5 machine config file
//   2. Read scalar fields (machine name, optical train count, working distance)
//   3. Inspect binary data shape (ClearBox correction grid)
//   4. Write the config to a temporary HDF5 file
//   5. Read the temporary file back
//   6. Assert round-trip fidelity and print PASS / FAIL
//
// No additional dependencies beyond the library itself.

import { existsSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, basename } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';

// ---------------------------------------------------------------------------
// Resolve the repo root so this script works regardless of working directory
// (this file lives at examples/quickstart/nodejs/, so the repo root is three
// levels up).
// ---------------------------------------------------------------------------
const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, '..', '..', '..');
const FIXTURE = join(REPO_ROOT, 'fixtures', 'reference_config.h5');

if (!existsSync(FIXTURE)) {
  console.error(`Fixture not found: ${FIXTURE}`);
  console.error('Run from the repo root or ensure fixtures/ is present.');
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Import the library. This uses the compiled output (dist/), the same way
// nodejs/src/cli.ts does, so this script has no build step of its own.
// ---------------------------------------------------------------------------
const distIndex = join(REPO_ROOT, 'nodejs', 'dist', 'index.js');
if (!existsSync(distIndex)) {
  console.error(`Build output not found: ${distIndex}`);
  console.error('Run "npm run build" inside nodejs/ first.');
  process.exit(1);
}
const { MachineConfigReader, MachineConfigWriter } = await import(pathToFileURL(distIndex).href);

async function run() {
  // -------------------------------------------------------------------------
  // Step 1 & 2 — Open the file and read scalar fields
  // -------------------------------------------------------------------------
  const reader = new MachineConfigReader(FIXTURE);
  const config = await reader.parse();

  console.log('=== Machine Config Quickstart ===\n');
  console.log(`Machine name   : ${config.meta.machine_name}`);
  console.log(`Optical trains : ${config.optical_trains.length}`);

  const train0 = config.optical_trains[0];
  const wd = train0.scanner.working_distance;
  const wdUnit = train0.scanner.working_distance_unit ?? '';
  console.log(`Working dist   : ${wd} ${wdUnit}   (train 0)`);

  // -------------------------------------------------------------------------
  // Step 3 — Binary data shape (correction grid)
  // -------------------------------------------------------------------------
  const correction = await reader.getCorrectionData(0); // { data: Float64Array, shape: [257,257,2] }
  console.log(`Correction grid: [${correction.shape.join(', ')}]   (train 0)`);

  // -------------------------------------------------------------------------
  // Step 4 — Write to a temporary file
  // -------------------------------------------------------------------------
  console.log();
  const tmpPath = join(tmpdir(), `machine_config_quickstart_${randomUUID()}.h5`);

  await new MachineConfigWriter(config).write(tmpPath);
  console.log(`Written to     : ${basename(tmpPath)}`);

  // -------------------------------------------------------------------------
  // Step 5 — Read the temporary file back
  // -------------------------------------------------------------------------
  const config2 = await new MachineConfigReader(tmpPath).parse();

  // -------------------------------------------------------------------------
  // Step 6 — Assert round-trip fidelity
  // -------------------------------------------------------------------------
  const failures = [];

  if (config2.meta.machine_name !== config.meta.machine_name) {
    failures.push(
      `  machine_name: expected ${JSON.stringify(config.meta.machine_name)}, ` +
        `got ${JSON.stringify(config2.meta.machine_name)}`,
    );
  }

  if (config2.optical_trains.length !== config.optical_trains.length) {
    failures.push(
      `  train_count: expected ${config.optical_trains.length}, got ${config2.optical_trains.length}`,
    );
  }

  const wd2 = config2.optical_trains[0].scanner.working_distance;
  if (wd2 !== wd) {
    failures.push(`  working_distance: expected ${wd}, got ${wd2}`);
  }

  // Clean up the temp file regardless of outcome.
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
