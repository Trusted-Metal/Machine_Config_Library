// AV-04: Reader returns typed error when required group is absent
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'missing_machine_group.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, 'no error raised for missing Machine/ group'];
  } catch (e) {
    return [true, `${(e as Error).constructor.name} raised for missing Machine/ group: ${e}`];
  }
}
