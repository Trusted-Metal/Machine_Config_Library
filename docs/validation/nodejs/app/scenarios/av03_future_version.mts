// AV-03: v1.0 reader encountering a v1.1 file fails loudly
import { join } from 'node:path';
import { MachineConfigReader, UnsupportedFileVersion } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'v1_1_simulated.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, "no error raised for File_Version='1.1'"];
  } catch (e) {
    if (e instanceof UnsupportedFileVersion) {
      if (e.version === '1.1') {
        return [true, `UnsupportedFileVersion raised, version='${e.version}'`];
      }
      return [false, `UnsupportedFileVersion raised but version='${e.version}'`];
    }
    return [false, `wrong exception: ${(e as Error).constructor.name}: ${e}`];
  }
}
