// S-08: Drastic change to real AconityMIDI file — new train, build_plate_x, rotation, clearbox cleared
import { randomUUID } from 'node:crypto';
import { globSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';

export async function run(_fixturesDir: string, realDir: string): Promise<[boolean, string]> {
  const matches = globSync('*.h5', { cwd: realDir })
    .filter((name) => name.includes('AconityMIDI') && name.includes('OG_178'));
  if (matches.length === 0) {
    return [false, `real AconityMIDI file not found in ${realDir}`];
  }

  const cfg = await new MachineConfigReader(`${realDir}/${matches[0]}`).parse();

  // Clone train 1 as train 3
  const newTrain = structuredClone(cfg.optical_trains[1]);
  newTrain.train_id = 'Optical_Train_03';
  newTrain.scanner = { ...newTrain.scanner, scan_head_rotation: 90.0 };
  newTrain.optional_components = { ...newTrain.optional_components, clearbox: null };

  const modified = {
    ...cfg,
    meta: { ...cfg.meta, machine_name: 'MODIFIED_ACONITY_VALIDATION' },
    machine: { ...cfg.machine, build_plate_x: 350.0, machine_name: 'MODIFIED_ACONITY_VALIDATION' },
    optical_trains: [...cfg.optical_trains, newTrain],
  };

  const tmp = join(tmpdir(), `s08_${randomUUID()}.h5`);
  await new MachineConfigWriter(modified).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.optical_trains.length !== 3) {
    return [false, `optical_trains: expected 3, got ${rb.optical_trains.length}`];
  }
  if (rb.machine.build_plate_x == null || Math.abs(rb.machine.build_plate_x - 350.0) > 0.001) {
    return [false, `build_plate_x: got ${rb.machine.build_plate_x}`];
  }
  const r2 = rb.optical_trains[2].scanner.scan_head_rotation;
  if (r2 == null || Math.abs(r2 - 90.0) > 0.001) {
    return [false, `train[2].scan_head_rotation: got ${r2}`];
  }
  if (rb.optical_trains[2].optional_components.clearbox !== null) {
    return [false, 'train[2].clearbox should be null'];
  }
  if (rb.meta.machine_name !== 'MODIFIED_ACONITY_VALIDATION') {
    return [false, `machine_name: got '${rb.meta.machine_name}'`];
  }
  if (rb.meta.file_version.trim() !== '1.0') {
    return [false, `file_version changed: '${rb.meta.file_version}'`];
  }

  return [true, '3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK'];
}
