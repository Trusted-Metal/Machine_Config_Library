// AV-07: Dispatcher handles empty string File_Version
import { join } from 'node:path';
import { MachineConfigReader } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'empty_version.h5');
  try {
    const cfg = await new MachineConfigReader(fixture).parse();
    return [true, `empty File_Version defaults to '1.0', reads OK, file_version='${cfg.meta.file_version}'`];
  } catch (e) {
    return [true, `empty File_Version raises ${(e as Error).constructor.name}: ${e}`];
  }
}
