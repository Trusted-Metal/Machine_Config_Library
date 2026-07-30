import { describe, it, expect } from 'vitest';
import { SCHEMA_VERSION, getSchema, validate } from '../src/index.js';

describe('Schema — live tests', () => {
  it('SCHEMA_VERSION is "v1"', () => {
    expect(SCHEMA_VERSION).toBe('v1');
  });

  it('loads the schema file without error', () => {
    expect(() => getSchema()).not.toThrow();
  });

  it('schema is a non-empty object', () => {
    const schema = getSchema();
    expect(typeof schema).toBe('object');
    expect(schema).not.toBeNull();
  });

  it('schema has top-level properties key', () => {
    const schema = getSchema();
    expect(schema).toHaveProperty('properties');
  });

  it('schema includes optical_trains property', () => {
    const schema = getSchema();
    const props = schema['properties'] as Record<string, unknown>;
    expect(props).toHaveProperty('optical_trains');
  });

  it('validate() returns errors for an empty object', () => {
    const errors = validate({});
    expect(errors.length).toBeGreaterThan(0);
  });
});
