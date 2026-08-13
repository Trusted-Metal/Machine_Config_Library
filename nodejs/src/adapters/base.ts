/**
 * Base interface for machine-config adapters.
 * Operates on the raw parsed JSON dict before deserialization into typed models.
 */
export interface Adapter {
  readonly name: string;
  adapt(config: Record<string, unknown>): Record<string, unknown>;
}
