// AV-08: File_Version string survives write→read unchanged
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config.h5');
  const cfg = await new MachineConfigReader(path).parse();
  const origVersion = cfg.meta.file_version.trim();

  const tmp = join(tmpdir(), `av08_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();
  const rbVersion = rb.meta.file_version.trim();

  if (rbVersion !== '1.0') {
    return [false, `file_version after roundtrip: expected '1.0', got '${rbVersion}'`];
  }
  if (rbVersion !== origVersion) {
    return [false, `file_version changed: '${origVersion}' → '${rbVersion}'`];
  }

  return [true, `File_Version survives roundtrip unchanged: '${rbVersion}'`];
}
