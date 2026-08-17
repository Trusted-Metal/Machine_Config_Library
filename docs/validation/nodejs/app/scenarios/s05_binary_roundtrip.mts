// S-05: Full binary round-trip with correction hash
import { createHash, randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';

function arraysEqual(a: Float64Array, b: Float64Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const av = a[i], bv = b[i];
    if (av !== bv && !(Number.isNaN(av) && Number.isNaN(bv))) return false;
  }
  return true;
}

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const reader = new MachineConfigReader(path);
  const cfg = await reader.parse({ includeBinary: true });

  const cdBefore = await reader.getCorrectionData(0);
  const icdBefore = await reader.getInverseCorrectionData(0);

  const tmp = join(tmpdir(), `s05_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const reader2 = new MachineConfigReader(tmp);

  const cdAfter = await reader2.getCorrectionData(0);
  const icdAfter = await reader2.getInverseCorrectionData(0);

  if (!arraysEqual(cdBefore.data, cdAfter.data)) {
    const hBefore = createHash('sha256').update(Buffer.from(cdBefore.data.buffer)).digest('hex').slice(0, 16);
    const hAfter = createHash('sha256').update(Buffer.from(cdAfter.data.buffer)).digest('hex').slice(0, 16);
    return [false, `correction_data mismatch: ${hBefore}... → ${hAfter}...`];
  }
  if (!arraysEqual(icdBefore.data, icdAfter.data)) {
    return [false, 'inverse_correction_data mismatch after roundtrip'];
  }

  const cdHash = createHash('sha256').update(Buffer.from(cdBefore.data.buffer)).digest('hex');
  return [true, `correction_data preserved: SHA-256=${cdHash}`];
}
