export type { Adapter } from './base.js';

import type { Adapter } from './base.js';

const _registry: Map<string, Adapter> = new Map();

/** Register an adapter by name. */
export function registerAdapter(adapter: Adapter): void {
  _registry.set(adapter.name, adapter);
}

/** Retrieve a registered adapter by name, or undefined if not found. */
export function getAdapter(name: string): Adapter | undefined {
  return _registry.get(name);
}

/** Return the ordered chain of all registered adapters. */
export function getChain(): Adapter[] {
  return Array.from(_registry.values());
}
