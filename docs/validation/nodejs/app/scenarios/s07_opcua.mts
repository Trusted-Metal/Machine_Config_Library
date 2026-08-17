// S-07: OPCUA configuration round-trip
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { MachineConfigReader, MachineConfigWriter } from 'machine-config-library';

export async function run(fixturesDir: string, _realDir: string): Promise<[boolean, string]> {
  const path = join(fixturesDir, 'reference_config_opcua.h5');
  if (!existsSync(path)) {
    return [false, `OPCUA fixture not found: ${path}`];
  }

  const cfg = await new MachineConfigReader(path).parse();
  if (cfg.opcua == null) {
    return [false, 'opcua is undefined after reading OPCUA fixture'];
  }

  const origUrl = cfg.opcua.client.server_url;
  const origTimeout = cfg.opcua.client.session_timeout;
  const origTriggersEnabled = cfg.opcua.triggers_enabled;
  const origTriggerNames = new Set(Object.keys(cfg.opcua.triggers));

  const tmp = join(tmpdir(), `s07_${randomUUID()}.h5`);
  await new MachineConfigWriter(cfg).write(tmp);
  const rb = await new MachineConfigReader(tmp).parse();

  if (rb.opcua == null) {
    return [false, 'opcua is undefined after roundtrip'];
  }
  if (rb.opcua.client.server_url !== origUrl) {
    return [false, `server_url changed: '${origUrl}' → '${rb.opcua.client.server_url}'`];
  }
  if (rb.opcua.client.session_timeout !== origTimeout) {
    return [false, `session_timeout changed: ${origTimeout} → ${rb.opcua.client.session_timeout}`];
  }
  if (rb.opcua.triggers_enabled !== origTriggersEnabled) {
    return [false, `triggers_enabled changed: ${origTriggersEnabled} → ${rb.opcua.triggers_enabled}`];
  }

  const rbNames = new Set(Object.keys(rb.opcua.triggers));
  if (rbNames.size !== origTriggerNames.size || ![...origTriggerNames].every((n) => rbNames.has(n))) {
    const missing = [...origTriggerNames].filter((n) => !rbNames.has(n));
    return [false, `trigger names changed: missing=${JSON.stringify(missing)}`];
  }

  const co = 'Chamber Oxygen Level';
  if (co in cfg.opcua.triggers) {
    const ot = cfg.opcua.triggers[co], rt = rb.opcua.triggers[co];
    if (ot.signal !== rt.signal || ot.subsystem !== rt.subsystem) {
      return [false, `'${co}' signal/subsystem changed`];
    }
  }

  return [true, `OPCUA roundtrip OK: ${origTriggerNames.size} triggers, url='${origUrl}'`];
}
