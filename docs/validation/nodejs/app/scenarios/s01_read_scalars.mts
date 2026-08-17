// S-01: Read reference fixture, all scalar fields
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const cfg = await new MachineConfigReader(join(fixturesDir, 'reference_config.h5')).parse();

  if (cfg.meta.machine_name !== 'TM-LPBF-02: AconityMIDI+_OG') {
    return [false, `machine_name: got '${cfg.meta.machine_name}'`];
  }
  if (cfg.machine.build_plate_x == null || Math.abs(cfg.machine.build_plate_x - 250.0) > 0.001) {
    return [false, `build_plate_x: got ${cfg.machine.build_plate_x}`];
  }
  if (cfg.machine.build_plate_y == null || Math.abs(cfg.machine.build_plate_y - 250.0) > 0.001) {
    return [false, `build_plate_y: got ${cfg.machine.build_plate_y}`];
  }
  if (cfg.optical_trains.length !== 2) {
    return [false, `optical_trains count: got ${cfg.optical_trains.length}`];
  }

  const wd = cfg.optical_trains[0].scanner.working_distance;
  if (wd == null || Math.abs(wd - 670.0) > 0.1) {
    return [false, `train[0].working_distance: got ${wd}`];
  }

  const r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
  if (r0 == null || Math.abs(r0) > 0.001) {
    return [false, `train[0].scan_head_rotation: got ${r0}`];
  }

  const r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
  if (r1 == null || Math.abs(r1 - 180.0) > 0.001) {
    return [false, `train[1].scan_head_rotation: got ${r1}`];
  }

  const h = cfg.meta.configuration_hash;
  if (h.length !== 64 || !/^[0-9a-fA-F]+$/.test(h)) {
    return [false, `configuration_hash invalid: '${h}'`];
  }
  if (cfg.meta.file_version.trim() !== '1.0') {
    return [false, `file_version: got '${cfg.meta.file_version}'`];
  }

  return [true, 'all scalar fields match expected values'];
}
