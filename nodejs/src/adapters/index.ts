export type { Adapter } from './base.js';

import type { Adapter } from './base.js';
import { V0_9_to_V1_0, fromVersion as _v09from, toVersion as _v09to } from './test_v0_9_to_v1_0.js';

// name-keyed registry (for getAdapter compatibility)
const _registry: Map<string, Adapter> = new Map();
// version-pair-keyed registry used by getChainFor
const _versionRegistry: Map<string, Adapter> = new Map();

function _register(adapter: Adapter, from: string, to: string): void {
  _registry.set(adapter.name, adapter);
  _versionRegistry.set(`${from}->${to}`, adapter);
}

// Register all generated adapters; add new entries here as adapters are created.
_register(new V0_9_to_V1_0(), _v09from, _v09to);

/** Register an adapter by name (public extension point). */
export function registerAdapter(adapter: Adapter): void {
  _registry.set(adapter.name, adapter);
}

/** Retrieve a registered adapter by name, or undefined if not found. */
export function getAdapter(name: string): Adapter | undefined {
  return _registry.get(name);
}

/**
 * Return the ordered adapter chain needed to migrate `fromVersion` → `toVersion`.
 * Returns an empty array when no migration is required or no path exists.
 * TODO: build a graph walk when multi-hop chains are needed.
 */
export function getChainFor(fromVersion: string, toVersion: string): Adapter[] {
  if (fromVersion === toVersion) return [];
  const adapter = _versionRegistry.get(`${fromVersion}->${toVersion}`);
  return adapter ? [adapter] : [];
}
