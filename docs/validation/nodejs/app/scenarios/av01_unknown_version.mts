// AV-01: Reader rejects unknown File_Version with typed error
import { join } from 'node:path';
import { MachineConfigReader, UnsupportedFileVersion } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const fixture = join(fixturesDir, '..', 'docs', 'validation', 'fixtures', 'v2_0_unknown.h5');
  try {
    await new MachineConfigReader(fixture).parse();
    return [false, "no error raised for File_Version='2.0'"];
  } catch (e) {
    if (e instanceof UnsupportedFileVersion) {
      if (e.version === '2.0') {
        return [true, `UnsupportedFileVersion raised, version='${e.version}'`];
      }
      return [false, `UnsupportedFileVersion raised but version='${e.version}'`];
    }
    return [false, `wrong exception: ${(e as Error).constructor.name}: ${e}`];
  }
}
