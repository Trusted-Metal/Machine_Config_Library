// S-03: Read real AconityMIDI fixture file
import { globSync } from 'node:fs';
import { MachineConfigReader } from 'machine-config-library';

export async function run(_fixturesDir: string, realDir: string): Promise<[boolean, string]> {
  const matches = globSync('*.h5', { cwd: realDir })
    .filter((name) => name.includes('AconityMIDI') && name.includes('OG_178'));
  if (matches.length === 0) {
    return [false, `real AconityMIDI file not found in ${realDir}`];
  }

  const cfg = await new MachineConfigReader(`${realDir}/${matches[0]}`).parse();
  const fields = [
    `machine_name=${JSON.stringify(cfg.meta.machine_name)}`,
    `file_version=${JSON.stringify(cfg.meta.file_version)}`,
    `trains=${cfg.optical_trains.length}`,
    `build_plate_x=${cfg.machine.build_plate_x}`,
    `build_plate_y=${cfg.machine.build_plate_y}`,
    `wd=${cfg.optical_trains[0].scanner.working_distance}`,
    `rotation[0]=${cfg.optical_trains[0].scanner.scan_head_rotation}`,
    `hash=${cfg.meta.configuration_hash.slice(0, 16)}...`,
  ].join(' | ');
  return [true, fields];
}
