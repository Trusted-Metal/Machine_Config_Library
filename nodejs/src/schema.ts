import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import type { ValidateFunction } from 'ajv';

// ajv v8 ships "export =" CJS types that are not constructable via ESM
// default import in NodeNext mode.  createRequire is the correct workaround.
// The schema uses draft 2020-12, so we import from ajv/dist/2020.
const _require = createRequire(import.meta.url);
type AjvInstance = { compile(schema: unknown): ValidateFunction };
const AjvClass: new (opts?: Record<string, unknown>) => AjvInstance = _require('ajv/dist/2020');

const __dirname = dirname(fileURLToPath(import.meta.url));

// Relative to compiled output: nodejs/dist/ → ../schema/
const SCHEMA_PATH = join(__dirname, '../schema/machine_config_v1.schema.json');

export const SCHEMA_VERSION = 'v1';

let _schema: Record<string, unknown> | null = null;
let _validate: ValidateFunction | null = null;

/**
 * Return the parsed JSON Schema object (loaded once, then cached).
 */
export function getSchema(): Record<string, unknown> {
  if (_schema === null) {
    _schema = JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')) as Record<string, unknown>;
  }
  return _schema;
}

/**
 * Validate data against the machine-config JSON Schema.
 *
 * Returns an array of human-readable error strings (empty if valid).
 */
export function validate(data: unknown): string[] {
  if (_validate === null) {
    const ajv = new AjvClass({ allErrors: true, strict: false });
    _validate = ajv.compile(getSchema());
  }
  const fn = _validate;
  const valid = fn(data);
  if (!valid) {
    return (
      fn.errors?.map(
        (e) => `${e.instancePath || '(root)'} ${e.message ?? 'unknown error'}`
      ) ?? []
    );
  }
  return [];
}
