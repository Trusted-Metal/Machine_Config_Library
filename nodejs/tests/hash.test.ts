import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeAll } from 'vitest';
import { MachineConfigReader } from '../src/index.js';
import { computeConfigurationHash, INTEGER_FIELD_NAMES } from '../src/hash.js';
import { getSchema } from '../src/schema.js';
import type { MachineConfig } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REFERENCE = join(__dirname, '../../fixtures/reference_config.h5');

let reference: MachineConfig;

beforeAll(async () => {
  reference = await new MachineConfigReader(REFERENCE).parse();
}, 60_000);

describe('computeConfigurationHash', () => {
  it('is stable and a 64-char lowercase hex digest', () => {
    const a = computeConfigurationHash(reference);
    const b = computeConfigurationHash(reference);
    expect(a).toBe(b);
    expect(a).toMatch(/^[0-9a-f]{64}$/);
  });

  it('ignores its own stored value', () => {
    const a = computeConfigurationHash(reference);
    const tampered: MachineConfig = {
      ...reference,
      meta: { ...reference.meta, configuration_hash: 'f'.repeat(64) },
    };
    const b = computeConfigurationHash(tampered);
    expect(a).toBe(b);
  });

  it('ignores export_date', () => {
    const a = computeConfigurationHash(reference);
    const changed: MachineConfig = {
      ...reference,
      meta: { ...reference.meta, export_date: '2099-01-01T00:00:00.000Z' },
    };
    const b = computeConfigurationHash(changed);
    expect(a).toBe(b);
  });

  it('ignores is_valid', () => {
    const a = computeConfigurationHash(reference);
    const withFlag: MachineConfig = {
      ...reference,
      meta: { ...reference.meta, is_valid: true },
    };
    const b = computeConfigurationHash(withFlag);
    expect(a).toBe(b);
  });

  it('changes when a scalar field changes', () => {
    const a = computeConfigurationHash(reference);
    const changed: MachineConfig = {
      ...reference,
      machine: {
        ...reference.machine,
        build_plate_x: (reference.machine.build_plate_x ?? 0) + 1,
      },
    };
    const b = computeConfigurationHash(changed);
    expect(a).not.toBe(b);
  });

  it('ignores binary grid data', async () => {
    const withBinary = await new MachineConfigReader(REFERENCE).parse({ includeBinary: true });
    const withoutBinary = await new MachineConfigReader(REFERENCE).parse({ includeBinary: false });
    expect(computeConfigurationHash(withBinary)).toBe(computeConfigurationHash(withoutBinary));
  });
});

// ---------------------------------------------------------------------------
// Schema cross-check for the integer-field allowlist
//
// hash.ts hardcodes INTEGER_FIELD_NAMES rather than reading
// schema/machine_config_v1.schema.json at runtime, since that schema is
// itself hand-maintained (not generated from the models) and has already
// been found to drift from other hand-maintained descriptions of it (see
// CONFIGURATION_HASH_PLAN.md's Step 3 notes: docs/schema.md claims OPCUA
// fields aren't part of the schema/export, but they are). This test is the
// safety net: it fails if hash.ts's hardcoded set and the schema's own
// "type": "integer" declarations ever disagree, so drift is caught by CI
// instead of silently producing a wrong hash.
// ---------------------------------------------------------------------------

describe('INTEGER_FIELD_NAMES — schema cross-check', () => {
  function collectLeafTypeNames(node: unknown, type: 'integer' | 'number', names: Set<string>): void {
    if (Array.isArray(node)) {
      for (const item of node) collectLeafTypeNames(item, type, names);
      return;
    }
    if (node === null || typeof node !== 'object') return;
    const obj = node as Record<string, unknown>;
    const props = obj.properties;
    if (props !== null && typeof props === 'object') {
      for (const [key, value] of Object.entries(props as Record<string, unknown>)) {
        if (value !== null && typeof value === 'object') {
          const t = (value as Record<string, unknown>).type;
          const types = Array.isArray(t) ? t : [t];
          if (types.includes(type)) names.add(key);
        }
      }
    }
    for (const value of Object.values(obj)) collectLeafTypeNames(value, type, names);
  }

  it('matches every "type": "integer" leaf name in the schema exactly', () => {
    const schema = getSchema();
    const schemaIntegerNames = new Set<string>();
    collectLeafTypeNames(schema, 'integer', schemaIntegerNames);

    expect(new Set(INTEGER_FIELD_NAMES)).toEqual(schemaIntegerNames);
  });

  it('never overlaps with a "type": "number" leaf name in the schema', () => {
    const schema = getSchema();
    const schemaNumberNames = new Set<string>();
    collectLeafTypeNames(schema, 'number', schemaNumberNames);

    for (const name of INTEGER_FIELD_NAMES) {
      expect(schemaNumberNames.has(name)).toBe(false);
    }
  });
});
