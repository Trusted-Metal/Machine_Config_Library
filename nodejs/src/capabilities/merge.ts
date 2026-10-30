import { SetMode } from './generated.js';

/** Deep clone via JSON (canonical config trees are JSON-safe). */
export function snapshot<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/**
 * Merge: copy keys from incoming that are not undefined onto a clone of current.
 * Nested plain objects are shallow-replaced per key (incoming object replaces that key).
 * Replace: return a snapshot of incoming only.
 */
export function applySetMode<T extends Record<string, unknown>>(
  current: T,
  incoming: T,
  mode: SetMode = SetMode.Merge,
): T {
  if (mode === SetMode.Replace) {
    return snapshot(incoming);
  }
  const out = snapshot(current) as Record<string, unknown>;
  for (const [key, value] of Object.entries(incoming)) {
    if (value !== undefined) {
      out[key] = value;
    }
  }
  return out as T;
}
