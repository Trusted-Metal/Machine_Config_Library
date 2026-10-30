/**
 * Capability-API error taxonomy (see schema/capabilities/errors.yaml).
 */
export type CapabilityErrorCode =
  | 'UnsupportedVersion'
  | 'UnsupportedInVersion'
  | 'NotPresent'
  | 'ValidationError'
  | 'IoError'
  | 'InvalidIndex'
  | 'Closed';

export interface CapabilityError {
  code: CapabilityErrorCode;
  message: string;
  /**
   * Names every individual violation at once (e.g. every missing required
   * OPCUA field) rather than only the first one encountered. Absent for
   * validation failures with nothing more specific to list.
   */
  details?: string[];
}

export function capabilityError(
  code: CapabilityErrorCode,
  message: string,
  details?: string[],
): CapabilityError {
  return details ? { code, message, details } : { code, message };
}

/** Programmer-bug errors — thrown, not returned in Result. */
export class SessionClosedError extends Error {
  constructor() {
    super('MachineConfigFile session is closed');
    this.name = 'SessionClosedError';
  }
}
