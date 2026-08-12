/**
 * Stable File_Version facade — open/create dispatch.
 */
import { err, ok, type Result } from './result.js';
import { capabilityError, type CapabilityError } from './errors.js';
import type { MachineConfigFile } from './generated.js';
import { MachineConfigFileV10 } from './v1_0.js';
import { MachineConfigReader } from '../reader.js';

export type { Result } from './result.js';
export { ok, err, isOk } from './result.js';
export type { CapabilityError, CapabilityErrorCode } from './errors.js';
export { capabilityError, SessionClosedError } from './errors.js';
export { SetMode } from './generated.js';
export type * from './generated.js';
export { MachineConfigFileV10 } from './v1_0.js';
export { applySetMode, snapshot } from './merge.js';

const SUPPORTED_CREATE = new Set(['1.0']);

export async function openMachineConfig(
  path: string,
): Promise<Result<MachineConfigFile, CapabilityError>> {
  try {
    const reader = new MachineConfigReader(path);
    const config = await reader.parse();
    const version = (config.meta.file_version || '1.0').trim() || '1.0';
    if (version === '1.0') {
      return MachineConfigFileV10.open(path);
    }
    return err(
      capabilityError(
        'UnsupportedVersion',
        `No capability adapter registered for File_Version "${version}"`,
      ),
    );
  } catch (e) {
    return err(capabilityError('IoError', e instanceof Error ? e.message : String(e)));
  }
}

export function createMachineConfig(
  version: string,
): Result<MachineConfigFile, CapabilityError> {
  const fv = version.trim() || '1.0';
  if (!SUPPORTED_CREATE.has(fv)) {
    return err(
      capabilityError(
        'UnsupportedVersion',
        `create() unsupported for File_Version "${fv}"`,
      ),
    );
  }
  return MachineConfigFileV10.create(fv);
}

export function supportedFileVersions(): string[] {
  return ['1.0'];
}
