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
}

export function capabilityError(
  code: CapabilityErrorCode,
  message: string,
): CapabilityError {
  return { code, message };
}

/** Programmer-bug errors — thrown, not returned in Result. */
export class SessionClosedError extends Error {
  constructor() {
    super('MachineConfigFile session is closed');
    this.name = 'SessionClosedError';
  }
}
