// S-04: Write modified config and verify field change survives round-trip
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const cfg = await new MachineConfigReader(path).parse();
  const origX = cfg.machine.build_plate_x;

  const modified = {
    ...cfg,
    meta: { ...cfg.meta, machine_name: 'VALIDATION_TEST_MACHINE' },
    machine: { ...cfg.machine, machine_name: 'VALIDATION_TEST_MACHINE' },
  };

  const tmp = join(tmpdir(), `s04_${randomUUID()}.h5`);
  await new MachineConfigWriter(modified).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.meta.machine_name !== 'VALIDATION_TEST_MACHINE') {
    return [false, `machine_name not persisted: '${rb.meta.machine_name}'`];
  }
  if (rb.meta.file_version.trim() !== '1.0') {
    return [false, `file_version changed: '${rb.meta.file_version}'`];
  }
  if (rb.machine.build_plate_x !== origX) {
    return [false, `build_plate_x changed: ${origX} → ${rb.machine.build_plate_x}`];
  }

  return [true, 'machine_name persisted, file_version and other fields unchanged'];
}
