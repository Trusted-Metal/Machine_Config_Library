// S-02: Read reference fixture, correction data
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const reader = new MachineConfigReader(join(fixturesDir, 'reference_config.h5'));
  const cd = await reader.getCorrectionData(0);
  const icd = await reader.getInverseCorrectionData(0);

  if (cd.shape.join(',') !== '257,257,2') {
    return [false, `correction_data shape: ${cd.shape.join(',')}`];
  }
  if (icd.shape.join(',') !== '257,257,2') {
    return [false, `inverse_correction_data shape: ${icd.shape.join(',')}`];
  }
  if (!cd.data.some((v) => Number.isFinite(v))) {
    return [false, 'correction_data: no finite values'];
  }
  if (!icd.data.some((v) => Number.isFinite(v))) {
    return [false, 'inverse_correction_data: no finite values'];
  }

  const identical = cd.data.length === icd.data.length &&
    cd.data.every((v, i) => v === icd.data[i] || (Number.isNaN(v) && Number.isNaN(icd.data[i])));
  if (identical) {
    return [false, 'correction_data and inverse_correction_data are identical'];
  }

  const cdHash = createHash('sha256').update(Buffer.from(cd.data.buffer)).digest('hex');
  return [true, `shapes OK, finite values OK, forward≠inverse, correction_data SHA-256=${cdHash}`];
}
