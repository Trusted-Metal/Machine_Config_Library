#!/usr/bin/env node
import { Command } from 'commander';
import { createRequire } from 'node:module';

const _require = createRequire(import.meta.url);
const { version } = _require('../package.json') as { version: string };

const program = new Command();

program
  .name('machine-config')
  .description('Machine Config Library — Node.js CLI')
  .version(version);

program
  .command('export-json <file>')
  .description('Export an HDF5 machine-config file to canonical JSON')
  .option('--indent <n>', 'JSON indentation (0 for compact)', '2')
  .option('--include-binary', 'Include correction_data and raw_bytes arrays')
  .action(async (file: string, opts: { indent: string; includeBinary?: boolean }) => {
    const { MachineConfigReader } = await import('./reader.js');
    const reader = new MachineConfigReader(file);
    const json = await reader.toJson({
      indent: parseInt(opts.indent, 10),
      includeBinary: opts.includeBinary ?? false,
    });
    process.stdout.write(json + '\n');
  });

program
  .command('write-hdf5 <json> <output>')
  .description('Write an HDF5 file from a canonical JSON file')
  .action(async (jsonPath: string, outputPath: string) => {
    const { readFileSync } = await import('node:fs');
    const { MachineConfigWriter } = await import('./writer.js');
    const data = JSON.parse(readFileSync(jsonPath, 'utf8'));
    const writer = new MachineConfigWriter(data);
    await writer.write(outputPath);
  });

program
  .command('correction-hash <file>')
  .description('Print the SHA-256 hash of a ClearBox correction grid (flat little-endian float64 bytes)')
  .option('--train <n>', '0-indexed optical train number', '0')
  .option('--inverse', 'Hash the inverse correction grid instead of the forward grid')
  .action(async (file: string, opts: { train: string; inverse?: boolean }) => {
    const { createHash } = await import('node:crypto');
    const { MachineConfigReader } = await import('./reader.js');
    const reader = new MachineConfigReader(file);
    const train = parseInt(opts.train, 10);
    const cd = opts.inverse
      ? await reader.getInverseCorrectionData(train)
      : await reader.getCorrectionData(train);

    // Explicit little-endian float64 bytes — cross-platform/cross-language
    // determinism, matching the Python and Rust correction-hash commands.
    const bytes = Buffer.allocUnsafe(cd.data.length * 8);
    for (let i = 0; i < cd.data.length; i++) {
      bytes.writeDoubleLE(cd.data[i], i * 8);
    }

    const digest = createHash('sha256').update(bytes).digest('hex');
    process.stdout.write(digest + '\n');
  });

program
  .command('copy-hdf5 <input> <output>')
  .description('Copy an HDF5 machine config file, preserving all binary data (correction grids, fc3 bytes)')
  .action(async (input: string, output: string) => {
    const { MachineConfigReader } = await import('./reader.js');
    const { MachineConfigWriter } = await import('./writer.js');
    const config = await new MachineConfigReader(input).parse({ includeBinary: true });
    await new MachineConfigWriter(config).write(output);
  });

program.parse();
