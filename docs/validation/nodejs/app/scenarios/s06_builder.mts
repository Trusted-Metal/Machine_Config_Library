// S-06: Build synthetic config with 2 lasers, verify fields and round-trip
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter, MockConfigBuilder } from 'machine-config-library';

export async function run(_fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const cfg = new MockConfigBuilder({ nLasers: 2 }).build();

  if (cfg.optical_trains.length !== 2) {
    return [false, `optical_trains count: ${cfg.optical_trains.length}`];
  }

  const r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
  const r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
  if (r0 == null || Math.abs(r0) > 0.001) {
    return [false, `train[0].scan_head_rotation: ${r0}`];
  }
  if (r1 == null || Math.abs(r1 - 180.0) > 0.001) {
    return [false, `train[1].scan_head_rotation: ${r1}`];
  }
  if (!cfg.meta.machine_name) {
    return [false, 'machine_name is empty'];
  }

  const cb = cfg.optical_trains[0].optional_components.clearbox;
  if (cb == null || cb.correction_data == null) {
    return [false, 'clearbox or correction_data is null'];
  }
  const center = cb.correction_data[128][128][0];
  if (center == null || !Number.isFinite(center) || Math.abs(center - 2.0) > 0.01) {
    return [false, `correction_data center: expected ~2.0, got ${center}`];
  }

  const tmp = join(tmpdir(), `s06_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.optical_trains.length !== 2) {
    return [false, `readback trains: ${rb.optical_trains.length}`];
  }
  if (rb.meta.machine_name !== cfg.meta.machine_name) {
    return [false, 'machine_name changed after roundtrip'];
  }

  return [true, '2-laser build OK, center≈2.0, roundtrip OK'];
}
