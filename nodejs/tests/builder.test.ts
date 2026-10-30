import { describe, it, expect, afterAll } from 'vitest';
import { join } from 'node:path';
import { unlinkSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { MockConfigBuilder, MachineConfigReader } from '../src/index.js';
import type { MachineConfig } from '../src/index.js';
import { clearbox, grid } from './clearboxHelpers.js';

/** Absolute path to a unique temp HDF5 file that does not exist yet. */
function tmpH5(): string {
  return join(tmpdir(), `test_builder_${randomBytes(8).toString('hex')}.h5`);
}

const tmpFiles: string[] = [];

afterAll(() => {
  for (const p of tmpFiles) {
    if (existsSync(p)) unlinkSync(p);
  }
});

/** Build + save + re-read, returning the re-parsed config and its output path. */
async function buildAndRead(
  builder: MockConfigBuilder,
): Promise<{ config: MachineConfig; path: string }> {
  const path = tmpH5();
  tmpFiles.push(path);
  await builder.save(path);
  const config = await new MachineConfigReader(path).parse();
  return { config, path };
}

// ===========================================================================
// build() — in-memory, no disk I/O
// ===========================================================================

describe('MockConfigBuilder — build()', () => {
  it('defaults to 2 optical trains', () => {
    const config = new MockConfigBuilder().build();
    expect(config.optical_trains.length).toBe(2);
  });

  it('honours nLasers: 1', () => {
    const config = new MockConfigBuilder({ nLasers: 1 }).build();
    expect(config.optical_trains.length).toBe(1);
  });

  it('defaults machine_name to "MockMachine"', () => {
    const config = new MockConfigBuilder().build();
    expect(config.meta.machine_name).toBe('MockMachine');
    expect(config.machine.machine_name).toBe('MockMachine');
  });

  it('defaults build plate dimensions to 250x250x20 mm', () => {
    const config = new MockConfigBuilder().build();
    expect(config.machine.build_plate_x).toBe(250.0);
    expect(config.machine.build_plate_y).toBe(250.0);
    expect(config.machine.build_plate_z).toBe(20.0);
  });

  it('honours custom build plate dimensions', () => {
    const config = new MockConfigBuilder({ buildPlateX: 300.0, buildPlateY: 280.0 }).build();
    expect(config.machine.build_plate_x).toBe(300.0);
    expect(config.machine.build_plate_y).toBe(280.0);
  });

  it('configuration_hash is exactly 64 characters', () => {
    const config = new MockConfigBuilder().build();
    expect(config.meta.configuration_hash).toHaveLength(64);
  });

  it('alternates scan_head_offset sign and rotation per train', () => {
    const config = new MockConfigBuilder({ nLasers: 2 }).build();
    const s0 = config.optical_trains[0].scanner;
    const s1 = config.optical_trains[1].scanner;
    expect(s0.scan_head_offset_x).toBeCloseTo(-87.5, 5);
    expect(s0.scan_head_offset_y).toBeCloseTo(23.5, 5);
    expect(s0.scan_head_rotation).toBe(0.0);
    expect(s1.scan_head_offset_x).toBeCloseTo(87.5, 5);
    expect(s1.scan_head_offset_y).toBeCloseTo(-23.5, 5);
    expect(s1.scan_head_rotation).toBe(180.0);
  });

  it('includeClearbox: true (default) populates ClearBox and SFCF on every train', () => {
    const config = new MockConfigBuilder({ nLasers: 2 }).build();
    config.optical_trains.forEach((train, i) => {
      expect(clearbox(config, i)).toBeDefined();
      expect(train.scan_field_correction_file).not.toBeNull();
    });
  });

  it('includeClearbox: false omits ClearBox and SFCF on every train', () => {
    const config = new MockConfigBuilder({ nLasers: 2, includeClearbox: false }).build();
    expect(config.extensions).toBeUndefined();
    for (const train of config.optical_trains) {
      expect(train.scan_field_correction_file).toBeNull();
    }
  });

  it('the correction grid in memory has shape [257, 257, 2] and a ~2.0 centre peak', () => {
    const config = new MockConfigBuilder({ nLasers: 1 }).build();
    const { data, shape } = grid(config, 0, 'Correction_Data');
    expect(shape).toEqual([257, 257, 2]);
    expect(data[(128 * 257 + 128) * 2]).toBeGreaterThan(1.9);
    expect(data[(128 * 257 + 128) * 2]).toBeLessThan(2.1);
  });
});

// ===========================================================================
// save() — write to HDF5, then verify via MachineConfigReader (round-trip)
// ===========================================================================

describe('MockConfigBuilder — save() + read back', () => {
  it('single laser round-trips to 1 optical train', async () => {
    const { config } = await buildAndRead(new MockConfigBuilder({ nLasers: 1 }));
    expect(config.optical_trains.length).toBe(1);
  });

  it('two lasers round-trip to 2 optical trains', async () => {
    const { config } = await buildAndRead(new MockConfigBuilder({ nLasers: 2 }));
    expect(config.optical_trains.length).toBe(2);
  });

  it('machine_name round-trips', async () => {
    const { config } = await buildAndRead(
      new MockConfigBuilder({ nLasers: 1, machineName: 'TestMachine' }),
    );
    expect(config.meta.machine_name).toBe('TestMachine');
  });

  it('correction grid round-trips with shape [257, 257, 2]', async () => {
    const { config } = await buildAndRead(new MockConfigBuilder({ nLasers: 1 }));
    expect(grid(config, 0, 'Correction_Data').shape).toEqual([257, 257, 2]);
  });

  it('correction grid centre peak is ~2.0 and the grid is non-zero', async () => {
    const { config } = await buildAndRead(new MockConfigBuilder({ nLasers: 1 }));
    const { data } = grid(config, 0, 'Correction_Data');
    const centre = data[(128 * 257 + 128) * 2];
    expect(centre).toBeGreaterThan(1.9);
    expect(centre).toBeLessThan(2.1);
    expect(data.some((v) => v !== 0)).toBe(true);
  });

  it('inverse correction grid is 0.9x the forward grid at centre', async () => {
    const { config } = await buildAndRead(new MockConfigBuilder({ nLasers: 1 }));
    const fwd = grid(config, 0, 'Correction_Data').data[(128 * 257 + 128) * 2];
    const inv = grid(config, 0, 'Inverse_Correction_Data').data[(128 * 257 + 128) * 2];
    expect(inv).toBeCloseTo(fwd * 0.9, 6);
  });

  it('no-clearbox path: config has no ClearBox or SFCF after round-trip', async () => {
    const { config } = await buildAndRead(
      new MockConfigBuilder({ nLasers: 1, includeClearbox: false }),
    );
    expect(config.extensions).toBeUndefined();
    expect(config.optical_trains[0].scan_field_correction_file).toBeNull();
  });

  it('round-trip output validates against the schema', async () => {
    const { path } = await buildAndRead(new MockConfigBuilder({ nLasers: 2 }));
    const { validate } = await import('../src/schema.js');
    const json = JSON.parse(await new MachineConfigReader(path).toJson());
    expect(validate(json)).toEqual([]);
  });
});
