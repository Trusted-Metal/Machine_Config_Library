import type { MachineConfig } from '../models.js';

/**
 * Base interface for machine-config adapters.
 *
 * An adapter transforms a MachineConfig into another representation
 * (e.g. a different schema or vendor-specific format).
 */
export interface Adapter {
  readonly name: string;
  adapt(config: MachineConfig): MachineConfig;
}
