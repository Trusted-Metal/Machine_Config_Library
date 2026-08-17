// AV-05: Reader handles corrupt required attribute gracefully
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'corrupt_scalar.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, 'no error raised for corrupt Build_Plate_X_Dimension'];
  } catch (e) {
    return [true, `${(e as Error).constructor.name} raised for corrupt scalar: ${e}`];
  }
}
