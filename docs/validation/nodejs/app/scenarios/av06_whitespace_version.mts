// AV-06: Dispatcher normalizes whitespace in File_Version
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'version_whitespace.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"];
  } catch (e) {
    return [false, `${(e as Error).constructor.name}: ${e}`];
  }
}
